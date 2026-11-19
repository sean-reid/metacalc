; Progress bar.

UB_INNER                .equ UI_WIDTH-6
UB_SIDES                .equ 5

; ui_bar   in:  A = text row 0 to 7, nothing drawn for a larger one; HL = done, DE = total,
;               both 0 to 65535
;          ORs the outline of Canvas::bar into the row from column 1 to 94 and a fill of
;          floor(90 * min(done, total) / total) columns from column 3 on three scanlines; the
;          fill is the full 90 when total is 0
;          destroys AF BC DE HL
ui_bar:
	cp UI_TEXT_ROWS
	ret nc
	push af
	ld b,d
	ld c,e
	ld a,b
	or c
	ld a,UB_INNER
	jr z,ub_fill
	or a
	sbc hl,bc
	jr nc,ub_fill
	add hl,bc
	ld d,h
	ld e,l
	xor a
	jr ub_n
#macro UB_DBL
	add hl,hl
	rla
#endmacro
#macro UB_DBLADD
	add hl,hl
	rla
	add hl,de
	adc a,0
#endmacro
ub_n:
	UB_DBL
	UB_DBLADD
	UB_DBLADD
	UB_DBL
	UB_DBLADD
	UB_DBL
	add hl,hl
	rla
	ld e,l
	ld l,h
	ld h,a
	ld d,2
ub_div:
	sla e
	adc hl,hl
	jr c,ub_over
	sbc hl,bc
	jr nc,ub_bit
	add hl,bc
ub_bit:
	rl d
	jr nc,ub_div
	ld a,d
	xor $7F
ub_fill:
	ld b,a
	pop af
	call ui_row_addr
	ld a,b
	add a,3
	ld b,a
	push hl
	and 7
	ld hl,iv_lmasks
	call ui_add_hl_a
	ld a,(hl)
	cpl
	ld e,a
	ld a,b
	rrca
	rrca
	rrca
	and $1F
	ld d,a
	pop hl
	push hl
	call ub_full
	ld b,UB_SIDES
ub_edges:
	call ub_side
	djnz ub_edges
	call ub_full
	pop hl
	ld a,2*UI_STRIDE
	call ui_add_hl_a
	call ub_fillrow
	call ub_fillrow

; One scanline of the fill, HL = its first byte, D = whole bytes of the run, E = the byte after
; them; HL = the next scanline's first byte out. Destroys AF BC.
ub_fillrow:
	ld b,d
	ld c,$1F
	ld a,b
	or a
	jr z,ub_last
ub_whole:
	ld a,c
	or (hl)
	ld (hl),a
	inc hl
	ld c,$FF
	djnz ub_whole
ub_last:
	ld a,e
	and c
	or (hl)
	ld (hl),a
	ld a,UI_STRIDE
	sub d
	jp ui_add_hl_a

ub_over:
	or a
	sbc hl,bc
	or a
	jr ub_bit

; A scanline of the outline from column 1 to 94, HL = its first byte; HL = the next scanline's
; first byte out. Destroys AF BC.
ub_full:
	ld a,$7F
	or (hl)
	ld (hl),a
	inc hl
	ld c,UI_STRIDE-2
ub_run:
	ld (hl),$FF
	inc hl
	dec c
	jr nz,ub_run
	ld a,$FE
	or (hl)
	ld (hl),a
	inc hl
	ret

; A scanline of the outline's two sides, HL = its first byte; HL = the next scanline's first
; byte out. Destroys AF.
ub_side:
	set 6,(hl)
	ld a,UI_STRIDE-1
	call ui_add_hl_a
	set 1,(hl)
	inc hl
	ret
