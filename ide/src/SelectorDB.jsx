import React from 'react';

var dbCounter = 0;

export default function SelectorDB({ IDEContext }) {
    function addDb() {
        dbCounter++;
        IDEContext.addTab("db:" + dbCounter);
    }
    return (
        <div>
            <div className='nino-ide-selector-title'>DB EXPLORER AN SOMETHING LONGER THAN THE LENGTH</div>
            <div >DBSelector</div>
            <button onClick={() => addDb()}>Add DB</button>
            <br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
            sdfsdfsdfsdfdsf<br/>
        </div>
    );
}