import React, { useEffect, useRef } from 'react';

export function Screen(props) {
  const {app} = props;
  // the actual canvas html element
  const canvasRef = useRef(null);

  const draw = (ctx) => {
    // the wasm code renders the display info into an ImageData object, which we just need to
    // pass to the canvas
    const data = app.display_data();
    if (data) {
      ctx.putImageData(data, 0, 0);
    }
  };

  useEffect(() => {
    const canvas = canvasRef.current;
    const ctx = canvas.getContext('2d');

    let animationFrameId;
    const render = () => {
      draw(ctx);
      animationFrameId = window.requestAnimationFrame(render);
    };

    // get things started
    render();

    // cleanup when the component is unmounted
    return () => {
      window.cancelAnimationFrame(animationFrameId)
    }

    // the draw function will be redefined after the effect runs, making it an endless loop, which
    // in this case is actually what we want
  }, [draw]);

  return <canvas className="screen" ref={canvasRef} {...props}/>;
}
