import React from 'react';
import Dialog from './Dialog';

export default function SelectorDB({ IDEContext }) {
    function addSetting() {
        IDEContext.addTab("setting:");
    }
    function newSetting() {
        IDEContext.addTab("setting:new setting");
    }
    return (
        <div>
            <div className='nino-ide-selector-title'>SETTINGS</div>
            <br />
            filter:
            <br />
            <input type="text" id="selector_settings_filter" name="filter settings" maxLength="1024" />
            <br />
            <br />
            <table className="nino_list_fixed_header">
                <thead>
                    <tr>
                        <th>Setting</th>
                        <th>Value</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td>nino_key 1</td>
                        <td>There should be the value of each property</td>
                    </tr>
                    <tr>
                        <td>nino_key 1</td>
                        <td>There should be the value of each property</td>
                    </tr>
                    <tr>
                        <td>nino_key 1</td>
                        <td>There should be the value of each property</td>
                    </tr>
                    <tr>
                        <td>nino_key 1</td>
                        <td>There should be the value of each property</td>
                    </tr>
                    <tr>
                        <td>nino_key 1</td>
                        <td>There should be the value of each property</td>
                    </tr>
                    <tr>
                        <td>nino_key 1</td>
                        <td>There should be the value of each property</td>
                    </tr>
                    <tr>
                        <td>nino_key 1</td>
                        <td>There should be the value of each property</td>
                    </tr>
                    <tr>
                        <td>nino_key 1</td>
                        <td>There should be the value of each property</td>
                    </tr>
                    <tr>
                        <td>nino_key 1</td>
                        <td>There should be the value of each property</td>
                    </tr>
                </tbody>
            </table>
            <br />

            <button onClick={() => newSetting()}>New</button>

            <Dialog>
                <span>from parent</span>
            </Dialog>
        </div>
    );
}