import React, { useRef } from 'react';


export default function Dialog({ children }) {
    const dialogRef = useRef(null);
    function close(e) {
        debugger;
        dialogRef.close();
    }
    return (
        <div className='nino-ide-dialog'>
            <div className='nino-ide-dialog-background'></div>
            <div className='nino-ide-dialog-content' ref={dialogRef}>
                text here <br />
                {children}<br />
                <button onClick={close}>close</button>

            </div>
        </div>
    );
};