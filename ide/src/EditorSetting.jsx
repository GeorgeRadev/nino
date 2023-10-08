import React from 'react';

export default function EditorSettings({ objectID }) {
    const settingKey = objectID.split(":")[1];
    const settingValueReference = React.useRef(null);

    function settingSave() {

    }
    function settingRefresh() {

    }

    const settingValue = settingRefresh();
    return (
        <div style={{ padding: "5px" }}>
            <button onClick={settingSave}>Save</button>&nbsp;&nbsp;&nbsp;
            <button onClick={settingRefresh}>Refresh</button>
            <hr />
            <div className='nino-ide-ui-container-50'>
                setting key: <br />
                <input type="text" className="nino-ide-editor-field" name="settings-key" value={settingKey} readOnly={true} maxLength="1024" />
            </div>
            <div className='nino-ide-ui-container-50'>
                setting value: <br />
                <input type="text" className="nino-ide-editor-field" name="settings-value" value={settingValue} ref={settingValueReference} maxLength="1024" />
            </div>
        </div>
    );
}