import React from 'react';

export function SpeedSlider({stepsPerTick, setStepsPerTick, min, max}) {
  return (
    <input type="range"
           className="form-range"
           style={{width: 'auto'}}
           min={min}
           max={max}
           value={stepsPerTick}
           onInput={(e) => setStepsPerTick(e.target.value)}/>
  );
}
