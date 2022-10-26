import React, { useEffect, useState} from 'react';
import { Screen } from './Screen.jsx';
import { Button } from './Button.jsx';
import { FilePicker } from './FilePicker.jsx';
import { SpeedSlider } from './SpeedSlider.jsx';
import { CodeView } from './CodeView.jsx'
import { MemoryWatchBlock } from './MemoryWatchBlock.jsx'

const readAllFiles = (fileNames) => {
  return new Promise((resolve, reject) => {
    // we only want to actually compile and load the bytecode files after everything has been
    // added to the internal file list in wasm
    let loadedFiles = new Map();

    for (let file of fileNames) {
      const reader = new FileReader();

      reader.onerror = reject;
      reader.onload = () => {
        const content = reader.result;

        loadedFiles.set(file.name, content);
        if (loadedFiles.size === fileNames.length) {
          // all files have been read, so return the map
          resolve(loadedFiles);
        }
      };

      reader.readAsText(file);
    }
  })
}

function ButtonRow({step, loadFiles, running, setRunning, programLoaded}) {
  return (
    <div id="control-buttons">
      <Button
        className="btn"
        disabled={!programLoaded}
        onClick={() => setRunning(!running)}>
        {running ? "Stop" : "Start"}
      </Button>
      <Button className="btn"
              disabled={running || !programLoaded}
              onClick={step}>
        Step
      </Button>
      <Button className="btn"
              disabled={!programLoaded}
              onClick={loadFiles}>
        Reset
      </Button>

    </div>);
}

const showError = alert;

// this needs to be separated from VmEmulator so that the app isn't recreated when this component
// gets recreated
export function VMEmulatorStepper({app}) {
  const minStepsPerTick = 500;
  const maxStepsPerTick = 100000;

  const [running, setRunning] = useState(false);
  const [stepsPerTick, setStepsPerTick] = useState((maxStepsPerTick - minStepsPerTick) / 2);

  const [files, setFiles] = useState(new Map());
  const [offset, setOffset] = useState(0);
  const [fileNames, setFileNames] = useState([]);
  const [activeFile, setActiveFile] = useState(null);
  const [activeFunction, setActiveFunction] = useState(null);

  const run_steps = () => {
    try {
      app.step_times(stepsPerTick);
    } catch(error) {
      setRunning(false);
      showError(error);
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

  useEffect(() => {
    const readFiles = async () => {
      setRunning(false);

      const files = await readAllFiles(fileNames);
      setFiles(files);

      // because we cannot easily pass a list to wasm, it's a lot easier to just make the wasm app
      // remember the files we already added. So we need to clear that memory when loading new files
      app.reset_files();
      for (const [fileName, fileContent] of files) {
        app.add_file(fileName, fileContent);
      }
      app.load_files()
    };

    readFiles().catch((error) => {
      showError(error);
      setFiles(new Map());
      setFileNames([]);
    });
  }, [fileNames]);

  // some stuff should only be enabled if a program has been loaded
  const programLoaded = files.size !== 0;

  const pickFiles = (e) => {
    setRunning(false);
    setFileNames(e.target.files);
  };

  const jumpToCurrentInstr = () => {
    const file = app.current_file_name();
    const func = app.current_function_name();
    const offset = app.current_file_offset();

    if (files.get(file)) {
      setActiveFile(file);
    }
    setActiveFunction(func);
    setOffset(offset);
  };

  const step = () => {
    try {
      app.step();
    }catch(error) {
      showError(error);
    }
    jumpToCurrentInstr();
  };

  const locals = Array.from(app.locals());
  const stack = Array.from(app.stack());
  const args = Array.from(app.args());

  return (
    <>
      <div className={`wrapper ${running ? 'running': ''}`}>
        <div id="toolbar">
          <FilePicker
            onChange={pickFiles}/>

          <ButtonRow
            loadFiles={() => app.load_files()}
            step={step}
            running={running}
            setRunning={(run) => {
              if (!run) {
                jumpToCurrentInstr();
              }
              setRunning(run);
            }}
            programLoaded={programLoaded}/>

          <div id="speed">
            <SpeedSlider
              min={minStepsPerTick}
              max={maxStepsPerTick}
              stepsPerTick={stepsPerTick}
              setStepsPerTick={setStepsPerTick}/>
          </div>
        </div>

        {
          // while running, we want the canvas to take as much space as possible
          !running &&
            <CodeView
              files={files}
              activeFileName={activeFile}
              functionName={activeFunction}
              activeLine={offset}/>
        }

        <div className="screen-wrapper">
          <Screen
            app={app}
            width="512"
            height="256"/>
        </div>

        {
          !running &&
            <div id="watches">
              <MemoryWatchBlock
                name="locals"
                vars={locals}/>
              <MemoryWatchBlock
                name="args"
                vars={args}/>
              <MemoryWatchBlock
                name="stack"
                vars={stack}/>
            </div>
        }
      </div>
    </>);
}
