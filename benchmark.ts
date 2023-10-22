import { Suite } from "asyncmark";

const suite = new Suite({ parallel: true });
const init: RequestInit = {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded;charset=UTF-8" },
    body: "salt=benchmark",
};

suite.add({
    number: 500,  // note: rate-limiting needs to be adjusted
    async fun () {
        const response = await fetch("http://127.0.0.1:8080/request", init);
        if (!response.ok) throw new Error("assertion statusCode === 200 failed");
        await response.text();
    }
});

suite.run();
