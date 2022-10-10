import {App, get_key_code} from "nand-to-tetris-web";

let steps_per_tick = 12000;

const vm_screen = document.getElementById('screen');
const ctx = vm_screen.getContext('2d');

const app = App.new();
let interval = null;

const words_per_row = 32;
const width_px = 512;
const height_px = 256;
const bytes_per_pixel = 4;
const data = new Uint8ClampedArray(width_px * height_px * bytes_per_pixel);

const render = (display_memory) => {
  let i = 0;

  // TODO: move this into rust
  for (let row_idx = 0; row_idx < height_px; row_idx++) {
    for (let word_idx = 0; word_idx < words_per_row; word_idx++) {
      const word = display_memory[row_idx * words_per_row + word_idx];
      for (let pixel_idx = 0; pixel_idx < 16; pixel_idx++) {
        const mask = 1 << pixel_idx;
        const value = word & mask;
        const color = value == 0 ? 255 : 0;

        data[i++] = color;
        data[i++] = color;
        data[i++] = color;
        data[i++] = 255;
      }
    }
  }

  let img_data = new ImageData(data, width_px, height_px);
  ctx.putImageData(img_data, 0, 0);
};


const run = () => {
  app.step_times(steps_per_tick);
};

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
stop_button.onclick = () => {
  console.log('pause');
  clearInterval(interval);
  interval = null;
};

const step_button = document.getElementById('step-button');
step_button.onclick = () => {
  console.log('step');
  app.step();
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
  const data = app.display_buffer();
  render(data);
  requestAnimationFrame(render_loop);
}

render_loop();
