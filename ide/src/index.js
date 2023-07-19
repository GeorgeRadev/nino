import React from 'react';
import ReactDOM from 'react-dom/client';
import IDE from './IDE';
import './index.css';

const root = ReactDOM.createRoot(document.getElementById('nino-ide'));
root.render(
  <React.StrictMode>
    <IDE />
  </React.StrictMode>
);
