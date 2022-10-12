import {App, get_key_code} from "nand-to-tetris-web";

const speed_slider = document.getElementById('speed-slider');
let steps_per_tick = speed_slider.value;

const vm_screen = document.getElementById('screen');
const ctx = vm_screen.getContext('2d');

const app = App.new();
let interval = null;

const showError = (error) => {
  clearInterval(interval);
  interval = null;
  alert(error);
};

const render = (img_data) => {
  ctx.putImageData(img_data, 0, 0);
};

const run = () => {
  try {
    app.step_times(steps_per_tick);
  } catch (error) {
    showError(error);
  }
};

const pause = () => {
  console.log('pause');
  clearInterval(interval);
  interval = null;
};

const handle_file_upload = (evt) => {
  pause();
  app.reset_files();

  const files = evt.target.files;
  let loaded = 0;

  for (let file of files) {
    const reader = new FileReader();
    reader.onload = (e) => {
      const content = e.target.result;
      app.add_file(file.name, content);
      if (++loaded == files.length) {
        try {
          app.load_files();
        } catch (error) {
          showError(error);
        }
      }
    }
    reader.readAsText(file);
  }
};
document.getElementById('upload').addEventListener('change', handle_file_upload);


const start_button = document.getElementById('start-button');
start_button.onclick = () => {
  console.log('starting');
  // already running, so don't do anything
  if (interval !== null) {
    return;
  }

  // if we just ran in a normal loop we would never receive any events
  interval = setInterval(run, 0);
};

const stop_button = document.getElementById('stop-button');
stop_button.onclick = pause;

const step_button = document.getElementById('step-button');
step_button.onclick = () => {
  console.log('step');
  app.step();
};

speed_slider.oninput= (e) => {
  steps_per_tick = e.target.value;
};

const handle_input = (key) => {
  // only set the key if the vm is currently running
  if (interval !== null) {
    app.set_input_key(key);
  }
};

document.addEventListener('keydown', ({key}) => {
  handle_input(get_key_code(key));
});

document.addEventListener('keyup', (e) => {
  handle_input(0);
});

const render_loop = () => {
  const data = app.display_data();
  render(data);
  requestAnimationFrame(render_loop);
}

render_loop();
