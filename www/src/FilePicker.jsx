import React from 'react';

export function FilePicker({onChange}) {
  return (
    <form encType="multipart/form-data"
          id="upload-files">
      <label className="btn"
             style={{cursor: 'pointer'}}>
        Upload Files
        <input type="file"
               hidden
               accept="text/plain,.vm"
               names="files[]" multiple
               onChange={onChange}/>
      </label>
    </form>);
}
