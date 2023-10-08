import React, { useState, useEffect } from 'react';
import MonacoEditor from 'react-monaco-editor';

export default function EditorDynamics({ objectID }) {
    const [code, setCode] = useState('');
    const [language, setLanguage] = useState('javascript');
    const options = {
        autoIndent: 'full',
        contextmenu: true,
        fontFamily: 'monospace',
        fontSize: 13,
        lineHeight: 16,
        hideCursorInOverviewRuler: true,
        matchBrackets: 'always',
        minimap: {
            enabled: false,
        },
        scrollbar: {
            horizontalSliderSize: 6,
            verticalSliderSize: 6,
        },
        selectOnLineNumbers: false,
        roundedSelection: false,
        readOnly: false,
        cursorStyle: 'line',
        automaticLayout: true,
    };

    function save() {

    }
    return (
        <div style={{
            height: "calc(100% - 20px)"
        }}>
            <div style={{
                paddingLeft: "30px"
            }}>
                <button onClick={() => save()}>Save</button>
            </div>
            <MonacoEditor
                height="100%"
                language={language}
                value={code}
                options={options}
            />
        </div>
    );
}