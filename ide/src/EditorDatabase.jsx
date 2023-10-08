import React from 'react';

export default function EditorDB({ objectID }) {
    return (
        <div>
            <div className='nino-ide-ui-container-50'>
                objectID: {objectID}<br />
            </div>
            <div className='nino-ide-ui-container-50'>
                right container

            </div>
            <div className='nino-ide-ui-container-50'>
                third container

            </div>
        </div>
    );
}