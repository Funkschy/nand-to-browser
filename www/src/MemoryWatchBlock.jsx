import React from 'react';

export function MemoryWatchBlock({name, vars}) {
  const id = name;
  const label = name.charAt(0).toUpperCase() + name.slice(1);

  return (
    <div className="watch-block">
      {
        vars.length > 0
          ? <label className="block-label" htmlFor={id}>{label}</label>
          : <></>
      }
      <div id={id}>
        {
          vars.map((value, index) => {
            return (
              <div className="watch" key={index}>
                <span className="watch-label">{index}</span>
                <span className="watch-content">{value}</span>
              </div>
            );
          })
        }
      </div>
    </div>
  );
}
