import React from 'react';
import Actions from './Actions';
import Selectors from './Selectors';
import Viewers from './Viewers';

// https://fonts.google.com/icons?icon.set=Material+Symbols

export default function IDE() {
  const IDEContext = {
    setSelectedAction: null,
  }
  return (
    <div id='nino-ide-container'>
      <Actions IDEContext={IDEContext} />
      <Selectors IDEContext={IDEContext} />
      <Viewers />
    </div>
  );
};