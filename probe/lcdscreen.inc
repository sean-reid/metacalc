; The screens of lcdcheck. Contract: the including file defines putchar-callable text through
; the OS, P_TMP, P_TMP2 and P_STR (printnum.inc) and S_T2.

; Contract: A is one character.
putchar:
	bcall(_PutC)
	ret

; Contract: A is the row; the cursor goes to its first column.
at_row:
	ld (curRow),a
	xor a
	ld (curCol),a
	ret

; Returns HL at the prompt of the case in S_CASE.
prompt_ptr:
	ld a,(S_CASE)
	add a,a
	ld e,a
	ld d,0
	ld hl,prompts
	add hl,de
	ld e,(hl)
	inc hl
	ld d,(hl)
	ex de,hl
	ret

; Contract: HL is a string, DE the address of a word. Prints the string, then the word as five digits.
; The OS's text calls may destroy every register but IY, so what is needed after one is kept on the
; stack or in scratch.
put_label_word:
	push de
	call putstr
	pop hl
	call load_word
	jp putdec

; Contract: HL is a counter word's address. Returns the word in HL.
load_word:
	ld e,(hl)
	inc hl
	ld d,(hl)
	ex de,hl
	ret

; Contract: HL is the first of two counter words and DE a label. Prints the label, the first word,
; a space and the second word, each as five digits.
put_pair:
	push hl
	ex de,hl
	call putstr
	pop hl
	push hl
	call load_word
	call putdec
	ld a,' '
	call putchar
	pop hl
	inc hl
	inc hl
	call load_word
	jp putdec

show_pass:
	bcall(_ClrLCDFull)
	xor a
	call at_row
	ld hl,txt_title
	call putstr
	ld a,(S_PASS)
	or a
	ld hl,txt_6
	jr z,sp_speed
	ld hl,txt_15
sp_speed:
	call putstr
	ld a,1
	call at_row
	ld hl,LCD_HIST_DATA
	ld de,txt_d01
	call put_pair
	ld a,2
	call at_row
	ld hl,LCD_HIST_DATA + 4
	ld de,txt_d23
	call put_pair
	ld a,3
	call at_row
	ld hl,txt_dtot
	ld de,LCD_TOTAL_DATA
	call put_label_word
	ld a,4
	call at_row
	ld hl,LCD_HIST_CMD
	ld de,txt_c01
	call put_pair
	ld a,5
	call at_row
	ld hl,LCD_HIST_CMD + 4
	ld de,txt_c23
	call put_pair
	ld a,6
	call at_row
	ld hl,txt_ctot
	ld de,LCD_TOTAL_CMD
	call put_label_word
	ld a,7
	call at_row
	ld hl,txt_any
	call putstr
	ld a,(S_IN20)
	jp hexbyte

show_case:
	bcall(_ClrLCDFull)
	xor a
	call at_row
	ld hl,txt_key
	call putstr
	ld a,(S_CASE)
	add a,a
	ld e,a
	ld d,0
	ld hl,names
	add hl,de
	ld e,(hl)
	inc hl
	ld d,(hl)
	ex de,hl
	call putstr
	ld hl,txt_code
	call putstr
	ld a,(S_KEY)
	call hexbyte
	ld a,1
	call at_row
	ld hl,txt_pre
	ld ix,S_PRE
	call put_slot
	ld a,2
	call at_row
	ld hl,txt_lcd
	ld ix,S_LCD
	call put_slot
	ld a,3
	call at_row
	ld hl,txt_plt
	ld ix,S_PLT
	call put_slot
	ld a,4
	call at_row
	ld hl,txt_r7
	ld ix,S_R7
	call put_slot
	ld a,7
	call at_row
	ld hl,txt_any_key
	jp putstr

; Contract: HL is a label, IX a result slot, which is kept in S_SLOTP. Prints the label, the count as three digits, and the row
; and column of the first difference as two digits each, or dashes.
put_slot:
	ld (S_SLOTP),ix
	call putstr
	ld ix,(S_SLOTP)
	ld l,(ix+0)
	ld h,(ix+1)
	call putdec3
	ld hl,txt_row
	call putstr
	ld ix,(S_SLOTP)
	ld a,(ix+2)
	call put_two
	ld hl,txt_col
	call putstr
	ld ix,(S_SLOTP)
	ld a,(ix+3)

; Contract: A is a number below 100, or FFh for two dashes. Prints two digits.
put_two:
	cp 0FFh
	jr nz,pt_digits
	ld a,'-'
	call putchar
	ld a,'-'
	jp putchar
pt_digits:
	ld b,'0'
pt_tens:
	cp 10
	jr c,pt_units
	sub 10
	inc b
	jr pt_tens
pt_units:
	ld (S_T2),a
	ld a,b
	call putchar
	ld a,(S_T2)
	add a,'0'
	jp putchar

#include "printnum.inc"

prompts:
	.dw txt_p0, txt_p1, txt_p2
names:
	.dw txt_n0, txt_n1, txt_n2

txt_title:
	.db "lcdchk v", '0' + VERSION, ' ', 0
txt_6:
	.db "6MHz", 0
txt_15:
	.db "15MHz", 0
txt_d01:
	.db "d01 ", 0
txt_d23:
	.db "d23 ", 0
txt_dtot:
	.db "dtot ", 0
txt_c01:
	.db "c01 ", 0
txt_c23:
	.db "c23 ", 0
txt_ctot:
	.db "ctot ", 0
txt_any:
	.db "any key in20=", 0
txt_any_key:
	.db "any key", 0
txt_key:
	.db "key ", 0
txt_code:
	.db " k=", 0
txt_pre:
	.db "pre ", 0
txt_lcd:
	.db "lcd ", 0
txt_plt:
	.db "plt ", 0
txt_r7:
	.db "r7  ", 0
txt_row:
	.db " r", 0
txt_col:
	.db " c", 0
txt_p0:
	.db "press 5", 0
txt_p1:
	.db "2nd then 5", 0
txt_p2:
	.db "ALPHA then 5", 0
txt_n0:
	.db "plain", 0
txt_n1:
	.db "2nd", 0
txt_n2:
	.db "alpha", 0

; test-check-8xk.sh edits the signature field in one hex record, so it must not start across two
	.db 0, 0
