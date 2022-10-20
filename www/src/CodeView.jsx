import React, { useEffect, useRef } from 'react';

function BytecodeLine({highlightLine, activeLine, children}) {
  const ref = useRef(null);

  useEffect(() => {
    if (ref.current && highlightLine) {
      ref.current.scrollIntoView({
        behavior: 'smooth',
        block: 'center'
      });
    }
  }, [activeLine]);

  return (
    <td className={highlightLine ? 'highlight' : ''} ref={ref} children={children}>
    </td>
  );
}

const makeBytecodeLines = (lineStrings, activeLineIndex) => {
  // we need to ignore lines with labels because the VM will give us an offset inside the compiled
  // bytecode, which does not contain any labels
  let indexWithoutLabels = 0;

  return lineStrings.map((line, index) => {
    let highlightLine = indexWithoutLabels === activeLineIndex;
    let key_index = indexWithoutLabels;

    if (line.startsWith('label') || line.startsWith('//')) {
      // just some index that cannot possibly be the activeLineIndex
      key_index = -index - 1;
      highlightLine = false;
    }else {
      indexWithoutLabels += 1;
    }

    return (
      <tr key={key_index}>
        <BytecodeLine
          activeLine={activeLineIndex}
          highlightLine={highlightLine}>
          {line}
        </BytecodeLine>
      </tr>
    );
  });
};

export function CodeView({files, activeFileName, functionName, activeLine}) {
  // find the source code position to highlight
  const activeCode = files.get(activeFileName);
  const lines = activeCode !== undefined ? activeCode.split('\n') : [];

  return (
    <div className="code">
      <div className="code-child">
        <div className="code-inner">
          <table >
            <thead>
              <tr>
                <th>{activeFileName} {functionName ? ': ' + functionName : ''}</th>
              </tr>
            </thead>
            <tbody>
              {makeBytecodeLines(lines, activeLine)}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
