// This file is part of www.nand2tetris.org
// and the book "The Elements of Computing Systems"
// by Nisan and Schocken, MIT Press.
// File name: projects/04/Mult.asm

// Multiplies R0 and R1 and stores the result in R2.
// (R0, R1, R2 refer to RAM[0], RAM[1], and RAM[2], respectively.)
//
// This program only needs to handle arguments that satisfy
// R0 >= 0, R1 >= 0, and R0*R1 < 32768.

// Put your code here.

    @R2
    M = 0 // schreibe 0 nach RAM[2], da dort in place addiert werden soll

(LOOP)
    @R0
    D = M // D = RAM[0], der zähler

    @END
    D;JLE // wenn D <= 0 sind wir fertig
    @R0
    M = M - 1 // zähler--

    @R1
    D = M

    @R2
    M = M + D

    @LOOP
    0;JMP

(END)
    @END
    0;JMP // endlosschleife
