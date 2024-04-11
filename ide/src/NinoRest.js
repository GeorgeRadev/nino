export default {
    settings: {
        "key1": "value1",
        "key2": "value2"
    },
    settingsGet: async function () {
        // const response = await fetch("/ide_rest?op=/settings/get");
        // const settings = await response.json();
        return this.settings;
    },
    settingGet: async function (setting_name) {
        // const response = await fetch("/ide_rest?op=/setting/get&name=" + setting_name);
        // const settings = await response.json();
        return this.settings[setting_name];
    },
    settingSet: async function (setting_name, setting_value) {
        // const response = await fetch("/ide_rest?op=/setting/set&name=" + setting_name);
        // const settings = await response.json();
        this.settings[setting_name] = setting_value;
    }
}