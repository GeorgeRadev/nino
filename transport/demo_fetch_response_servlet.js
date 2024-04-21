import {fetch_response} from '_fetch';

export default async function servlet() {
    return fetch_response(
        "https://e7.pngegg.com/pngimages/87/386/png-clipart-legacy-of-kain-defiance-blood-omen-2-legacy-of-kain-soul-reaver-soul-reaver-2-nosgoth-soul-reaver-2-game-computer-wallpaper-thumbnail.png",
        {
            method: "GET",
            timeout: 20000
        }
    );
}