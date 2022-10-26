import React from 'react';

export function MemoryWatchBlock({name, vars}) {
  // only display the block if it's actually needed
  if (vars.length === 0) {
    return <></>;
  }

  const id = name;
  const label = name.charAt(0).toUpperCase() + name.slice(1);

  return (
    <div className="watch-block">
      <label className="block-label" htmlFor={id}>{label}</label>
      <div id={id}>
        {
          vars.map((value, index) => {
            return (
              <div key={index}>
                <div className="watch with-tooltip">
                  <span className="watch-label">{index}</span>
                  <span className="watch-content">{value}</span>
                </div>
                {
                  value.length > 12 &&
                    <span className="tooltip">{value}</span>
                }
              </div>
            );
          })
        }
      </div>
    </div>
  );
}
