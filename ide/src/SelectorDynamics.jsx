import React from 'react';
import Dialog from './Dialog';

export default function SelectorDB({ IDEContext }) {
    const [visible, setVisible] = React.useState(false);

    function onOk() {
        IDEContext.addTab("dynamic:new setting");
    }
    function onClose() {
        setVisible(false);

    }
    function newSetting() {

        IDEContext.addTab("dynamic:new setting");
        //setVisible(true);
    }
    return (
        <div>
            <div className='nino-ide-selector-title'>DYNAMICS</div>
            <br />
            filter:
            <br />
            <input type="text" id="selector_settings_filter" name="filter settings" maxLength="1024" />
            <br />
            <br />
            <table className="nino_list_fixed_header">
                <thead>
                    <tr>
                        <th>Dynamic</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td>nino_key 1</td>
                    </tr>
                    <tr>
                        <td>nino_key 1</td>
                    </tr>
                    <tr>
                        <td>nino_key 1</td>
                    </tr>
                    <tr>
                        <td>nino_key 1</td>
                    </tr>
                </tbody>
            </table>
            <br />

            <button onClick={() => newSetting()}>New</button>

            <Dialog visible={visible} onOk={() => onOk()} onClose={() => onClose()} >
                <span>name of dynamic : <br />
                    <input list="browsers" />
                    <datalist id="browsers">
                        <option value="Internet Explorer" />
                        <option value="Firefox" />
                        <option value="Google Chrome" />
                        <option value="Opera" />
                        <option value="Safari" />
                    </datalist>
                </span>
            </Dialog >
        </div >
    );
}