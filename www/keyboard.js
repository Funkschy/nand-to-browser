const newline_key = 128;
const backspace_key = 129;
const left_key = 130;
const up_key = 131;
const right_key = 132;
const down_key = 133;
const home_key = 134;
const end_key = 135;
const page_up_key = 136;
const page_down_key = 137;
const insert_key = 138;
const delete_key = 139;
const esc_key = 140;
const f1_key = 141;
const f2_key = 142;
const f3_key = 143;
const f4_key = 144;
const f5_key = 145;
const f6_key = 146;
const f7_key = 147;
const f8_key = 148;
const f9_key = 149;
const f10_key = 150;
const f11_key = 151;
const f12_key = 152;

const action_key_codes = {
  'PageUp': page_up_key,
  'PageDown': page_down_key,
  'End': end_key,
  'Home': home_key,
  'ArrowLeft': left_key,
  'ArrowUp': up_key,
  'ArrowRight': right_key,
  'ArrowDown': down_key,
  'F1': f1_key,
  'F2': f2_key,
  'F3': f3_key,
  'F4': f4_key,
  'F5': f5_key,
  'F6': f6_key,
  'F7': f7_key,
  'F8': f8_key,
  'F9': f9_key,
  'F10': f10_key,
  'F11': f11_key,
  'F12': f12_key,
  'Insert': insert_key,
  'Backspace': backspace_key,
  'Enter': newline_key,
  'Escape': esc_key,
  'Delete': delete_key
};

// javascript translation of Definitions.getkeyCode in the official Tools
// TODO: move this into rust
export const get_key_code = ({key, keyCode}) => {
  let ret_key = 0;
  const letter = key;
  const code = keyCode;

  if (letter.length !== 1) {
    ret_key = action_key_codes[letter];
  } else if (code >= 65 && code <= 90) {
    ret_key = code;
  } else {
    ret_key = letter.charCodeAt(0);
  }

  return ret_key;
};
