import fetch from '_fetch';

export default async function servlet() {
    const response = await fetch(
        "https://worldtimeapi.org/api/timezone/Etc/UTC",
        {
            method: "GET",
            timeout: 20000,
            headers: {
                "Host": "worldtimeapi.org"
            },
            body: ""
        }
    );
    return response.json();
}