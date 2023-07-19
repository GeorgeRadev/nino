import React from 'react';

function RequestsSelector({ IDEContext }) {
  return (
    <div>
      <div className='nino-ide-selector-title'>RequestsSelector</div>
      <div >RequestsSelector body</div>
    </div>
  );
}

function DBSelector({ IDEContext }) {
  return (
    <div>
      <div className='nino-ide-selector-title'>DB EXPLORER AN SOMETHING LONGER THAN THE LENGTH</div>
      <div >DBSelector</div>
    </div>
  );
}

function ToggableSelector({ name, IDEContext, children }) {
  const selectedAction = IDEContext.selectedAction;
  return (
    <div style={{ display: selectedAction === name ? "block" : "none" }}>
      {children}
    </div>
  );
}

export default function Viewers({ IDEContext }) {
  const [repaint, repaintViewers] = React.useState();
  IDEContext.repaintViewers = repaintViewers
  IDEContext.repaint = repaint
  return (
    <div id='nino-ide-selectors'>
      <ToggableSelector name="requests" IDEContext={IDEContext}>
        <RequestsSelector IDEContext={IDEContext} />
      </ToggableSelector>

      <ToggableSelector name="databases" IDEContext={IDEContext}>
        <DBSelector IDEContext={IDEContext} />
      </ToggableSelector>
    </div>
  );
};