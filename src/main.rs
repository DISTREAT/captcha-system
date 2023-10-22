use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::middleware::Logger;
use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use config_file::FromConfigFile;
use env_logger::Env;
use log::error;
use lazy_static::lazy_static;
use magick_rust::{bindings, magick_wand_genesis, DrawingWand, MagickWand, PixelWand};
use rand::Rng;
use serde::Deserialize;
use std::cmp;
use std::time::{SystemTime, UNIX_EPOCH};

lazy_static! {
    static ref CONFIG: Config =
        Config::from_config_file("config.toml").expect("cannot parse config file: config.toml");
    static ref WIDTH_WITHOUT_RIGHT_PADDING: usize = CONFIG.captcha.width - CONFIG.captcha.padding_x;
    static ref HEIGHT_WITHOUT_BOTTOM_PADDING: usize =
        CONFIG.captcha.height - CONFIG.captcha.padding_y;
}

#[derive(Deserialize)]
struct ConfigServer {
    host: String,
    port: u16,
    governor_per_second: u64,
    governor_burst_size: u32,
}

#[derive(Deserialize)]
struct ConfigCaptcha {
    secret: String,
    code_length: usize,
    expire_eta: f64,
    format: String,
    width: usize,
    height: usize,
    background_color: String,
    text_color: String,
    font: String,
    font_size: f64,
    padding_x: usize,
    padding_y: usize,
    character_spacing: usize,
    rotation_limit: (isize, isize),
    swirl: isize,
    noise: f64,
}

#[derive(Deserialize)]
struct Config {
    server: ConfigServer,
    captcha: ConfigCaptcha,
}

async fn generate_digest(offset: isize, salt: &String) -> blake3::Hash {
    let mut hasher = blake3::Hasher::new();
    let unix_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("cannot retrieve system time")
        .as_secs() as f64;

    hasher.update(salt.as_bytes());
    hasher.update(CONFIG.captcha.secret.as_bytes());
    hasher.update(
        &((unix_time / CONFIG.captcha.expire_eta) + (offset as f64))
            .floor()
            .to_be_bytes(),
    );

    hasher.finalize()
}

async fn draw_captcha(code: &str) -> anyhow::Result<Vec<u8>> {
    let mut background_color = PixelWand::new();
    background_color.set_color(&CONFIG.captcha.background_color)?;

    let mut text_color = PixelWand::new();
    text_color.set_color(&CONFIG.captcha.text_color)?;

    let mut captcha_letter_wand = DrawingWand::new();
    captcha_letter_wand.set_fill_color(&text_color);
    captcha_letter_wand.set_font(&CONFIG.captcha.font)?;
    captcha_letter_wand.set_font_size(CONFIG.captcha.font_size);

    let mut mw = MagickWand::new();
    mw.new_image(
        CONFIG.captcha.width,
        CONFIG.captcha.height,
        &background_color,
    )?;

    let mut rng = rand::thread_rng();
    let mut last_position_x: usize = CONFIG.captcha.padding_x;

    for (index, character) in code.chars().enumerate() {
        last_position_x = rng.gen_range(
            last_position_x
                + if index >= 1 {
                    CONFIG.captcha.character_spacing
                } else {
                    0
                }
                ..cmp::min(
                    *WIDTH_WITHOUT_RIGHT_PADDING,
                    last_position_x
                        + ((*WIDTH_WITHOUT_RIGHT_PADDING - last_position_x) / (code.len() - index)),
                ),
        );

        mw.annotate_image(
            &captcha_letter_wand,
            last_position_x as f64,
            rng.gen_range(CONFIG.captcha.padding_y..*HEIGHT_WITHOUT_BOTTOM_PADDING) as f64,
            rng.gen_range(CONFIG.captcha.rotation_limit.0..CONFIG.captcha.rotation_limit.1) as f64,
            &character.to_string(),
        )?;
    }

    unsafe {
        bindings::MagickSwirlImage(
            mw.wand,
            CONFIG.captcha.swirl as f64,
            bindings::PixelInterpolateMethod_SplineInterpolatePixel,
        );
        bindings::MagickAddNoiseImage(
            mw.wand,
            bindings::NoiseType_ImpulseNoise,
            CONFIG.captcha.noise,
        );
    }

    mw.set_image_colorspace(bindings::ColorspaceType_GRAYColorspace)?;
    mw.set_compression(bindings::CompressionType_Group4Compression)?;
    mw.set_compression_quality(40)?;

    Ok(mw.write_image_blob(&CONFIG.captcha.format)?)
}

#[derive(Deserialize)]
struct RequestCaptcha {
    salt: String,
}

#[post("/request")]
async fn request(form: web::Form<RequestCaptcha>) -> impl Responder {
    if form.salt.len() == 0 {
        return HttpResponse::BadRequest().into();
    }

    let code = &generate_digest(0, &form.salt).await.to_hex()[0..CONFIG.captcha.code_length];
    let captcha = draw_captcha(code).await;

    match captcha {
        Ok(captcha_image) => HttpResponse::Ok()
            .insert_header(("Content-Type", format!("image/{}", CONFIG.captcha.format)))
            .body(captcha_image.to_vec()),
        Err(_) => {
            error!("{:?}", captcha);
            HttpResponse::InternalServerError().into()
        },
    }
}

#[derive(Deserialize)]
struct VerifyCaptcha {
    salt: String,
    code: String,
}

#[post("/verify")]
async fn verify(form: web::Form<VerifyCaptcha>) -> impl Responder {
    if form.salt.len() == 0 || form.code.len() == 0 || form.code.len() > CONFIG.captcha.code_length
    {
        return HttpResponse::BadRequest().into();
    }

    for index in (-1..1).rev() {
        if generate_digest(index, &form.salt).await.to_hex()[0..form.code.len()] == form.code {
            return HttpResponse::Ok()
                .insert_header(("Content-Type", "application/json"))
                .body("{\"valid\": true}");
        }
    }

    HttpResponse::Ok()
        .insert_header(("Content-Type", "application/json"))
        .body("{\"valid\": false}")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    assert!(CONFIG.captcha.secret.len() >= 32);
    assert!(CONFIG.captcha.padding_x < CONFIG.captcha.width);
    assert!(CONFIG.captcha.character_spacing < CONFIG.captcha.width);
    assert!(CONFIG.captcha.padding_y < CONFIG.captcha.height);

    magick_wand_genesis();

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let governor_conf = GovernorConfigBuilder::default()
        .per_second(CONFIG.server.governor_per_second)
        .burst_size(CONFIG.server.governor_burst_size)
        .finish()
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .wrap(Governor::new(&governor_conf))
            .wrap(Logger::default())
            .service(request)
            .service(verify)
    })
    .bind((CONFIG.server.host.clone(), CONFIG.server.port))?
    .run()
    .await
}
