import React from 'react';

export default function EditorStatics({ objectID }) {
    const requestPath = objectID.split(":")[1];
    var requestName;
    const requestNameReference = React.useRef(null);

    function settingSave() {

    }
    function settingRefresh() {
        requestName = "name";
    }

    settingRefresh();
    return (
        <div style={{ padding: "5px" }}>
            <button onClick={settingSave}>Save</button>&nbsp;&nbsp;&nbsp;
            <button onClick={settingRefresh}>Refresh</button>
            <hr />
            <div className='nino-ide-ui-container-100'>
                request path: <br />
                <input type="text" className="nino-ide-editor-field" name="request-name" value={requestPath} readOnly={true} maxLength="1024" />
            </div>
            <div className='nino-ide-ui-container-100'>
                to resource name: <br />
                <input type="text" className="nino-ide-editor-field" name="settings-value" value={requestName} ref={requestNameReference} maxLength="1024" />
            </div>
        </div>
    );
}