import React from 'react';
import Actions from './Actions';
import Selector from './Selector';
import Editor from './Editor';

export default function IDE() {
  const IDEContext = {
    setSelectedAction: null,
    tabs: []
  }
  return (
    <div id='nino-ide-container'>
      <Actions IDEContext={IDEContext} />
      <Selector IDEContext={IDEContext} />
      <Editor IDEContext={IDEContext} />
    </div>
  );
};