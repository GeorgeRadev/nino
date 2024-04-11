import fetch from '_fetch';

export default async function servlet() {
    debugger;

    const response = await fetch(
        "https://worldtimeapi.org/api/timezone/Etc/UTC",
        {
            method: "GET",
            body: "",
            timeout: 20000,
            headers: {
                "Host": "worldtimeapi.org"
            }
        }
    );
    return response.json();
}