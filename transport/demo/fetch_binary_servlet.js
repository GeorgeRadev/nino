import {fetch_binary} from '_fetch';

export default async function servlet() {
    const response = await fetch_binary(
        "https://i.pinimg.com/originals/fa/a9/77/faa977755e5ac96f40a9055ba5f122a5.png",
        {
            method: "GET",
            timeout: 20000
        }
    );
    return response;
}