import React from 'react';

var pathCounter = 0;

export default function SelectorRequests({ IDEContext }) {
    function addPath() {
        pathCounter++;
        IDEContext.addTab("path:" + pathCounter);
      }
      return (
        <div>
          <div className='nino-ide-selector-title'>RequestsSelector</div>
          <div >RequestsSelector body</div>
          <button onClick={() => addPath()}>Add Path</button>
        </div>
      );
}