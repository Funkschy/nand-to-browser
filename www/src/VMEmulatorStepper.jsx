import React, { useEffect, useState} from 'react';
import { Screen } from './Screen.jsx';
import { Button } from './Button.jsx';
import { FilePicker } from './FilePicker.jsx';
import { SpeedSlider } from './SpeedSlider.jsx';

const handle_file_upload = (app, evt) => {
  // because we cannot easily pass a list to wasm, it's a lot easier to just make the wasm app
  // remember the files we already added. So we need to clear that memory when loading new files
  app.reset_files();

  const files = evt.target.files;
  // we only want to actually compile and load the bytecode files after everything has been
  // added to the internal file list in wasm
  let loaded = 0;

  for (let file of files) {
    const reader = new FileReader();
    reader.onload = (e) => {
      const content = e.target.result;
      app.add_file(file.name, content);

      if (++loaded == files.length) {
        // all files have been read and added to the internal buffer
        app.load_files();
      }
    }
    reader.readAsText(file);
  }
};

function ScreenContainer(props) {
  return (
    <div className="horizontal-container canvas-container">
      <Screen width="512" height="265" {...props}/>
    </div>
  )
}

function ButtonRow({loadFiles, running, setRunning}) {
  return (
    <div className="horizontal-container">
      <Button
        className="btn"
        onClick={() => setRunning(!running)}>
        {running ? "Stop" : "Start"}
      </Button>
      <Button className="btn"
              onClick={loadFiles}>
        Reset
      </Button>
    </div>);
}

// this needs to be separated from VmEmulator so that the app isn't recreated when this component
// gets recreated
export function VMEmulatorStepper({app}) {
  const minStepsPerTick = 500;
  const maxStepsPerTick = 30000;

  const [running, setRunning] = useState(false);
  const [stepsPerTick, setStepsPerTick] = useState((maxStepsPerTick - minStepsPerTick) / 2);

  const run_steps = () => {
    try {
      app.step_times(stepsPerTick);
    } catch(error) {
      setRunning(false);
      alert(error);
    }
  };

  useEffect(() => {
    if (running) {
      // run the run_steps function as fast as possible
      const interval = setInterval(run_steps, 0);
      // when either running or stepsPerTick change, this component will be re-rendered
      // when that happens, this function will be run and clear the interval, which will be
      // re-created while creating the new component
      return () => {
        clearInterval(interval)
      };
    }
  }, [running, stepsPerTick]);

  const pickFiles = (e) => {
    setRunning(false);
    try {
      handle_file_upload(app, e);
    }catch (error) {
      alert(error);
    }
  }

  return (
    <div className="vertical-container">
      <div className="horizontal-container">
        <FilePicker
          onChange={pickFiles}/>
        <SpeedSlider
          min={minStepsPerTick}
          max={maxStepsPerTick}
          stepsPerTick={stepsPerTick}
          setStepsPerTick={setStepsPerTick}/>
      </div>

      <ScreenContainer
        app={app}/>

      <ButtonRow
        loadFiles={() => app.load_files()}
        running={running}
        setRunning={setRunning}/>
    </div>);
}
