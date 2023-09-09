import React from 'react';


export default function Dialog({ children, visible, onClose, onOk }) {
    function close() {
        onClose();
    } function ok() {
        onOk();
        onClose();
    }
    return (
        <div className="nino-ide-dialog-background" style={{ display: (visible) ? "block" : "none" }}>
            <div className="nino-ide-dialog-content">
                {children}
                <hr />
                <button onClick={() => close()}>close</button>
                <button onClick={() => ok()}>ok</button>
            </div>
        </div>
    );
};