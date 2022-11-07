// This file is part of www.nand2tetris.org
// and the book "The Elements of Computing Systems"
// by Nisan and Schocken, MIT Press.
// File name: projects/04/Fill.asm

// Runs an infinite loop that listens to the keyboard input.
// When a key is pressed (any key), the program blackens the screen,
// i.e. writes "black" in every pixel;
// the screen should remain fully black as long as the key is pressed. 
// When no key is pressed, the program clears the screen, i.e. writes
// "white" in every pixel;
// the screen should remain fully clear as long as no key is pressed.

// Put your code here.

(CHECK)
    @24576
    D = M
    @FILL_SCREEN_WHITE
    D;JEQ
    @FILL_SCREEN_BLACK
    0;JMP

    @CHECK
    0;JMP

(FILL_SCREEN_WHITE)
    @5
    M = 0
    @FILL_SCREEN
    0;JMP
(FILL_SCREEN_BLACK)
    @5
    M = -1 // 0xFFFF
    @FILL_SCREEN
    0;JMP

(FILL_SCREEN)
    @8192 // 256 rows * 32 words
    D = A // the number of 16 bit values to write

    @3
    M = D // save the counter in R3

(LOOP)
    @3
    D = M
    @CHECK
    D;JLE     // if counter <= 0 goto END

    @3
    D = M     // D = counter
    D = D - 1 // the counter is added to the screen offset, so we have to use D - 1 to be correct

    @SCREEN
    A = A + D
    D = A     // load the correct address into D
    @4
    M = D     // save the address in R4

    @5
    D = M     // load the color
    @4
    A = M
    M = D     // RAM[SCREEN + counter - 1] = 65535


    @3
    M = M - 1 // counter--
    @LOOP
    0;JMP
