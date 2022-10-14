import React from 'react';

export function FilePicker({onChange}) {
  return (
    <form encType="multipart/form-data">
      <label className="btn">
        Upload Files
        <input type="file"
               accept="text/plain,.vm"
               names="files[]" multiple
               onChange={onChange}/>
      </label>
    </form>);
}
