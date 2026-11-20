; The session every public routine runs in. The stub of a routine loads IX with its body and
; jumps to fm_enter; a body ends with A = status and B = its result at fm_leave, or at fm_fail
; for a status with no result.

fm_open:
	ld ix,open_body
	jr fm_enter

fm_statement:
	ld ix,statement_body
	jr fm_enter

fm_assertion:
	ld ix,assertion_body
	jr fm_enter

fm_select:
	ld ix,select_body
	jr fm_enter

fm_rows:
	ld ix,rows_body
	jr fm_enter

fm_nodes:
	ld ix,nodes_body
	jr fm_enter

fm_fold:
	ld ix,fold_body
	jr fm_enter

fm_folded:
	ld ix,folded_body
	jr fm_enter

; fm_cursor takes the node in A, which fm_enter's interrupt reads destroy and which no register
; keeps while interrupts are on: the stub pushes AF on the caller's stack, makes the same two reads
; and the di, and only then pops it into AF', where no interrupt can reach it.
fm_cursor:
	push af
	ld a,i
	jp pe,fc_on
	ld a,i
	jp pe,fc_on
	di
	exx
	ld b,0
	jr fc_go
fc_on:
	di
	exx
	ld b,1
fc_go:
	pop af
	ex af,af'
	ld ix,cursor_body
	jr fe_save

fm_draw:
	ld ix,draw_body

; in: IX = body, BC, DE and HL = the routine's arguments, held in the shadow registers while the
; ports change. The window from the port 5 write to the stack switch, and the one from the stack
; restore to the port 5 restore in fm_leave, hold no stack instruction: the caller's stack is in
; the page being swapped out.
fm_enter:
	ld a,i
	jp pe,fe_on
	ld a,i
	jp pe,fe_on
	di
	exx
	ld b,0
	jr fe_save
fe_on:
	di
	exx
	ld b,1
fe_save:
	in a,(5)
	ld c,a
	ld a,KERNEL_RAM_PAGE
fe_window:
	out (5),a
	ld a,b
	ld (fv_iff),a
	ld a,c
	ld (fv_p5),a
	ld (fv_sp),sp
fe_window_end:
	ld sp,FM_STACK_TOP
	exx
	push hl
	push de
	ld hl,(kv_ready)
	ld de,FM_OWNED
	or a
	sbc hl,de
	jr z,fe_own
	ld hl,0
	ld (fv_open),hl
fe_own:
	ld (kv_ready),de
	pop de
	pop hl
	jp (ix)

; in: A = status, B = result. Restores port 5, SP and the interrupt state, and sets the zero flag
; exactly when A is 0.
fm_leave:
	ld c,a
	ld a,(fv_iff)
	ld d,a
	ld a,(fv_p5)
	ld sp,(fv_sp)
fl_window:
	out (5),a
fl_window_end:
	ld a,d
	or a
	ld a,c
	jr z,fl_off
	or a
	ei
	ret
fl_off:
	or a
	ret

; in: A = status
fm_fail:
	ld b,0
	jr fm_leave

; out: Z when fm is open, else NZ and A = FM_ERR_NOT_OPEN. Keeps BC, DE and HL.
fm_need:
	ld a,(fv_open)
	cp FM_OPEN_MAGIC & $FF
	jr nz,fn_closed
	ld a,(fv_open+1)
	cp FM_OPEN_MAGIC >> 8
	ret z
fn_closed:
	ld a,FM_ERR_NOT_OPEN
	or a
	ret

; in: A = a reader status. out: A = the fm status for it. Keeps BC, DE and HL.
fm_reader_status:
	cp LIB_ERR_CLOSED
	jr z,fr_closed
	cp LIB_ERR_INDEX
	jr z,fr_index
	cp LIB_ERR_NONE
	jr z,fr_none
	cp LIB_ERR_GLYPHS
	jr z,fr_font
	ld a,FM_ERR_READ
	ret
fr_closed:
	ld a,FM_ERR_CLOSED
	ret
fr_index:
	ld a,FM_ERR_INDEX
	ret
fr_none:
	ld a,FM_ERR_NO_MAP
	ret
fr_font:
	ld a,FM_ERR_FONT
	ret
