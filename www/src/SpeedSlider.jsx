import React from 'react';

export function SpeedSlider({stepsPerTick, setStepsPerTick, min, max}) {
  return (
    <input type="range"
           min={min}
           max={max}
           value={stepsPerTick}
           onInput={(e) => setStepsPerTick(e.target.value)}/>
  );
}
