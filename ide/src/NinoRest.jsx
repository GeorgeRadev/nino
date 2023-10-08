export default class NinoREST {
    async settingsGet() {
        const response = await fetch("/ide_rest?op=/settings/get");
        const settings = await response.json();
        return settings;
    }
    async settingGet(setting_name) {
        const response = await fetch("/ide_rest?op=/setting/get&name=" + setting_name);
        const settings = await response.json();
        return settings;
    }
    async settingSet(setting_name, setting_value) {
        const response = await fetch("/ide_rest?op=/setting/set&name=" + setting_name);
        const settings = await response.json();
        return settings;
    }
}