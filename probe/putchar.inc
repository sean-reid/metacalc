; Contract: A is one character, 0Ah for a new line, 0Dh for the start of the line, 0Ch to clear.
putchar:
	cp 0Ah
	jr z, newline
	cp 0Dh
	jr z, home
	cp 0Ch
	jr z, clear
	bcall(_PutC)
	ret
newline:
	bcall(_NewLine)
	ret
home:
	xor a
	ld (curCol), a
	ret
clear:
	bcall(_ClrLCDFull)
	xor a
	ld (curRow), a
	ld (curCol), a
	ret
