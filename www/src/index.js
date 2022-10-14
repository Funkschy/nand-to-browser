import { App, get_key_code } from "nand-to-tetris-web";

import React, { useEffect, useRef, useState, Component } from 'react';
import { createRoot } from 'react-dom/client';

import './style.css';

import { VMEmulatorStepper } from './VMEmulatorStepper.jsx';

function VMEmulator() {
  const app = App.new();

  const handle_input = (key) => {
    app.set_input_key(key);
  };

  document.addEventListener('keydown', ({key}) => {
    handle_input(get_key_code(key));
  });

  document.addEventListener('keyup', () => {
    handle_input(0);
  });

  return <VMEmulatorStepper app={app}/>;
}

const container = document.getElementById('app');
const root = createRoot(container);
root.render(<VMEmulator/>);
