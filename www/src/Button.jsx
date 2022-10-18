import React from 'react';

export function Button({children, onClick, disabled}) {
  return (
    <button className="btn"
            disabled={disabled}
            onClick={(e) => {
              onClick(e);
              // unfocus the button
              // otherwise when the user clicks the button and then presses space,
              // the button would be clicked again instead of giving the input to the game
              e.target.blur();
            }}>
      {children}
    </button>
  );
}
