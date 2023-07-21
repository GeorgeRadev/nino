import React from 'react';
import Actions from './Actions';
import Selectors from './Selectors';
import Editors from './Editors';

export default function IDE() {
  const IDEContext = {
    setSelectedAction: null,
    tabs: []
  }
  return (
    <div id='nino-ide-container'>
      <Actions IDEContext={IDEContext} />
      <Selectors IDEContext={IDEContext} />
      <Editors IDEContext={IDEContext} />
    </div>
  );
};