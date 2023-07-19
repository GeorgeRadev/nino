import React from 'react';

function ViewerTabs() {
  return (
    <div className='nino-ide-viewer-tabs'>ViewerTabs</div>
  );
}


function Viewer() {
  return (
    <div className='nino-ide-viewer'>Viewer</div>
  );
}

export default function Viewers() {
  return (
    <div id='nino-ide-viewers'>
      <ViewerTabs />
      <Viewer />

      <div>
        <span className="material-symbols-outlined">favorite</span>
        <span className="material-symbols-outlined size-20">favorite</span>
        <span className="material-symbols-outlined size-32">favorite</span>
        <br/>
        <span className="material-symbols-outlined size-48">publish</span>
        <span className="material-symbols-outlined size-48">add_box</span>
        <span className="material-symbols-outlined size-48">check_box</span>
        <span className="material-symbols-outlined size-48">more_horiz</span>
        <span className="material-symbols-outlined size-48">menu</span>
        <span className="material-symbols-outlined size-48">keyboard_arrow_down</span>
        <span className="material-symbols-outlined size-48">keyboard_arrow_right</span>
        <span className="material-symbols-outlined size-48">keyboard_double_arrow_right</span>
        <span className="material-symbols-outlined size-48">keyboard_double_arrow_down</span>
        
        
      </div>
    </div>
  );
};