# CAPTCHA-System

This repository contains a functional CAPTCHA server as described in the
[Rotkeappchen CAPTCHA design proposal](https://github.com/DISTREAT/Rotkeappchen)
for generating and verifying CAPTCHA challenges.

This project was started in favor of implementing a full-fledged and faster version of [captcha-generator](https://github.com/DISTREAT/captcha-generator)
for use as a microservice.

## Building and Serving

1. Clone the repository
2. Edit the configuration file `config.toml` (Make sure to change the secret)
3. Build the docker image using `docker build -t captcha-system .`
4. Serve using `docker run -d --restart unless-stopped -p 8080:80 --name captcha-server captcha-system:latest`

## API Reference

```
POST /request HTTP/1.1
Accept: */*
Accept-Encoding: gzip, deflate
Connection: keep-alive
Content-Length: 29
Content-Type: application/x-www-form-urlencoded; charset=utf-8
Host: 127.0.0.1:8080
User-Agent: HTTPie/3.2.1

salt=request-idenfifying+salt


HTTP/1.1 200 OK
content-length: 9440
content-type: image/webp
date: Sun, 22 Oct 2023 14:12:23 GMT
```
---
```
POST /verify HTTP/1.1
Accept: */*
Accept-Encoding: gzip, deflate
Connection: keep-alive
Content-Length: 39
Content-Type: application/x-www-form-urlencoded; charset=utf-8
Host: 127.0.0.1:8080
User-Agent: HTTPie/3.2.1

salt=request-idenfifying+salt&code=8750


HTTP/1.1 200 OK
content-length: 15
content-type: application/json
date: Sun, 22 Oct 2023 14:14:00 GMT

{
    "valid": true
}
```

