; Kind 9, the Dist screen as Dist in crates/mcui/src/screens/dist.rs draws it: the label on the left of the title bar and "dv N" on
; the right, N being the assertion's disjoint pair count in decimal, the pairs "x,y" set in the math font and wrapped over six rows,
; "none" centred on row 3 when there is no pair, and five blank keys. UP and DOWN move the first row shown by one, CLEAR goes back to
; the theorem screen or the too large screen that F2 pushed it from (theorem_key in screen_theorem_key.inc, theorem_push_kind in
; screen_theorem.inc). Every other key stays.
;
; The pairs are the record's first list, which lib.z80 describes as lr_dnpairs and lr_dpairs: two symbol ids a pair, the variables the
; statement itself requires, which is Lib::dist's list for the model. The second list of a theorem, the pairs of its dummy variables, is
; not shown. mcpack stores each pair with the lower id first and the list sorted by id (mm/parse.rs), and the screen shows them in that
; order, which is not the order of their tokens' text.
;
; Frame: byte 1 the stage, bytes 2 and 3 the assertion as the theorem screen's frame holds them, bytes 4 and 5 the first row shown. The
; first row is two bytes: a row holds a pair at least and the pair count is 16 bits, so the rows can pass 255.
;
; Layout, as place in crates/mcglyph/src/layout.rs lays the model's pairs out. A pair is its first token, a comma and its second token,
; a column between each, so it is the width of the three plus 2 columns. The first pair starts row 0 at x 0. A later pair goes 3 columns
; after the width of the row it follows when that width plus 3 plus its own is at most DIST_WIDTH, 94, else it starts the next row at
; x 0; a pair wider than the row sits alone. A token is drawn at 1 column (MARGIN) plus its x on text row 1 + r, r being its row less
; the first row shown, for the six rows from the first. A token's glyph codes and width come from the reader's glyph map: far_lib_glyph
; writes the codes to FM_SCRATCH and far_ui_text_width measures them, as fm measures a token (lt_measure in fm/statement.inc). Placing a
; pair takes two of each. Drawing one takes two more far_lib_glyph and three far_ui_text: the first token, the comma one column after
; it and the second token one column after the comma. fm's tables are in bank C inside fm's own sessions and no fm routine returns a
; symbol's codes, so the screen calls nothing of fm, closes nothing and leaves selected and loaded what the theorem screen had: CLEAR
; runs the theorem screen's enter, which loads the assertion and fits it again.
;
; The pairs are read DIST_RUN, 16, at a time with far_lib_read into DV_BUF, and the screen keeps no table of them: dist_walk places
; each pair as it is read. dist_enter walks every pair once with nothing drawn to count the rows, DV_ROWS, which the key needs to know
; the last first row, the rows less 6 and 0 for fewer. dist_draw walks again, skips the pairs of the rows above the first shown, draws
; the six rows and stops at the first pair of the row after them, whose widths it has read.
;
; Derived state, in APP_TEXT from DV_ASSERT to DV_VARS_END, rebuilt by every enter and every draw: DV_ASSERT the assertion,
; DV_NPAIRS its pair count, DV_ROWS the rows (dist_enter only), DV_TOP the first row shown (dist_draw), DV_MODE 0 to count or 1
; to draw, DV_ST the status of dist_open in the draw, DV_ROW the row the walk is on, FFFFh before the first pair, DV_LW the
; width of that row, DV_LEFT the pairs not yet read, DV_PTR the library offset of the next, DV_BUFN the pairs in DV_BUF not yet
; taken and DV_BUFP the next of them, DV_WC the comma's width, DV_WA the first token's width, DV_X the x of the pair being
; placed and, while dist_ink draws it, of the token drawn next, DV_IDA and DV_IDB its symbols, DV_TR its text row, DV_COMMA the
; comma's glyph code and a 0, DV_RIGHT the title's right text and DV_CUT its left text cut to the room, DV_BUF the run of pairs.
; DV_LABEL, the label before it is cut, is in FM_SCRATCH + 64, and every glyph read and its terminating 0 are in FM_SCRATCH from
; its start, which holds nothing between calls. What else writes APP_TEXT while the screen is open or covered: the OS's
; overwrite of saveSScreen at an auto power down, which takes APP_TEXT, and a notice above the screen, which only the error
; notice can be, pushed when an enter of this screen fails after a power down or a pop, and which writes its own title; every
; one of them ends in the enter of this screen, which derives DV_ROWS again, so the key never reads a lost count. The first row
; lives in the frame and survives both. The key writes bytes 4 and 5 of the frame and nothing else, and writes nothing when it
; answers SCR_STAY.
;
; Registers: each routine below may destroy AF BC DE HL and IX.
; dist_enter   in: HL = the frame. Counts the rows. out: A = the reader's status, 0 for good, with Z as A: far_lib_record's from
;              dist_open, far_lib_read's from dist_next, far_lib_glyph's from dist_token. A status makes the shell push the error notice.
; dist_key     in: A = the key, HL = the frame. out: A = SCR_POP for CLEAR; SCR_REDRAW for DOWN when the first row is under DV_ROWS
;              less 6, which is 0 for six rows or fewer, and for UP when it is not 0, with the new first row written to the frame;
;              else SCR_STAY with nothing written.
; dist_draw    in: HL = the frame. Draws the title, the rows or "none", and the keys. The draw has no way to answer: a refusal of
;              far_lib_record leaves the right of the title empty and draws no row and no "none", a refusal of far_lib_label an empty
;              label, a refusal of far_lib_read or far_lib_glyph stops the walk with the rows before that pair drawn and, for a
;              refused second token, the first token and the comma of that pair drawn too. The next key or enter that is refused
;              pushes the notice.
; Interrupts and speed: every routine returns with both as at the call, and the screen writes no port 20h and executes no DI. The
; reader's calls and the blit turn interrupts off for their own length and give the caller's state back.
; Stack, in bytes written below the stack pointer at the entry, the caller's return address counted, with interrupts masked, measured
; and asserted exactly in dist_depth.rs on the stub OS and in dist_real.rs on the reference OS: dist_key 2 on every path; dist_enter 22
; and dist_draw 26; the F2 that pushes the screen 8 in theorem_key. An interrupt that lands in a call adds the handler's bytes.
; Calls: far_lib_record, far_lib_read, far_lib_glyph and far_lib_label through list_label_text, far_ui_text_width, far_ui_text,
; far_ui_clear, far_ui_truncate, far_ui_title, far_ui_text_centered, far_ui_soft_keys, and app_stage_block, app_strcpy, app_dec16 and
; list_room on this page.
; Cost, in T-states at 6 MHz on the reference OS: COSTS.

DIST_ROWS	.equ 6
DIST_WIDTH	.equ 94
DIST_RUN	.equ 16

DV_ASSERT	.equ APP_TEXT
DV_NPAIRS	.equ DV_ASSERT + 2
DV_ROWS		.equ DV_NPAIRS + 2
DV_TOP		.equ DV_ROWS + 2
DV_MODE		.equ DV_TOP + 2
DV_ST		.equ DV_MODE + 1
DV_ROW		.equ DV_ST + 1
DV_LW		.equ DV_ROW + 2
DV_LEFT		.equ DV_LW + 2
DV_PTR		.equ DV_LEFT + 2
DV_BUFN		.equ DV_PTR + 3
DV_BUFP		.equ DV_BUFN + 1
DV_WC		.equ DV_BUFP + 2
DV_WA		.equ DV_WC + 2
DV_X		.equ DV_WA + 2
DV_IDA		.equ DV_X + 2
DV_IDB		.equ DV_IDA + 1
DV_TR		.equ DV_IDB + 1
DV_COMMA	.equ DV_TR + 1
DV_RIGHT	.equ DV_COMMA + 2
DV_CUT		.equ DV_RIGHT + 9
DV_BUF		.equ DV_CUT + LIST_LABEL_MAX + 2
DV_VARS_END	.equ DV_BUF + (2 * DIST_RUN)
#if DV_VARS_END > (APP_TEXT + APP_TEXT_SIZE)
 .error "the Dist screen's variables are longer than APP_TEXT"
#endif
DV_LABEL	.equ FM_SCRATCH + 64

#ifdef LIBRARY
; In: HL = the frame.
dist_enter:
	push hl
	pop ix
	ld l, (ix + 2)
	ld h, (ix + 3)
	ld (DV_ASSERT), hl
	call dist_open
	ret nz
	xor a
	ld (DV_MODE), a
	call dist_walk
	ret nz
	ld hl, (DV_ROW)
	inc hl
	ld (DV_ROWS), hl
	xor a
	ret

; In: A = the key, HL = the frame.
dist_key:
	cp KEY_CLEAR
	jr z, dsk_pop
	cp KEY_DOWN
	jr z, dsk_down
	cp KEY_UP
	jr nz, dsk_stay
	ld bc, 4
	add hl, bc
	ld e, (hl)
	inc hl
	ld d, (hl)
	ld a, d
	or e
	jr z, dsk_stay
	dec de
	jr dsk_move
dsk_down:
	ld bc, 4
	add hl, bc
	ld e, (hl)
	inc hl
	ld d, (hl)
	inc de
	ld bc, (DV_ROWS)
	ld a, c
	sub DIST_ROWS
	ld c, a
	ld a, b
	sbc a, 0
	ld b, a
	jr c, dsk_stay
	ld a, c
	sub e
	ld a, b
	sbc a, d
	jr c, dsk_stay
dsk_move:
	ld (hl), d
	dec hl
	ld (hl), e
	ld a, SCR_REDRAW
	ret
dsk_stay:
	ld a, SCR_STAY
	ret
dsk_pop:
	ld a, SCR_POP
	ret

; In: HL = the frame.
dist_draw:
	push hl
	pop ix
	ld l, (ix + 2)
	ld h, (ix + 3)
	ld (DV_ASSERT), hl
	ld l, (ix + 4)
	ld h, (ix + 5)
	ld (DV_TOP), hl
	call far_ui_clear
	ld hl, dist_block
	ld bc, DIST_BLOCK_SIZE
	call app_stage_block
	call dist_open
	ld (DV_ST), a
	call dist_title
	ld a, (DV_ST)
	or a
	jr nz, dsd_keys
	ld hl, (DV_NPAIRS)
	ld a, h
	or l
	jr nz, dsd_rows
	ld hl, STG_DV_NONE
	ld b, 3
	call far_ui_text_centered
	jr dsd_keys
dsd_rows:
	ld a, 1
	ld (DV_MODE), a
	call dist_walk
dsd_keys:
	ld hl, STG_DV_KEYS
	jp far_ui_soft_keys

; Builds the title and draws it: the label, cut to the room "dv N" leaves, and "dv N" on the right, or
; nothing on the right when DV_ST is not 0.
dist_title:
	ld hl, (DV_ASSERT)
	ld de, DV_LABEL
	call list_label_text
	or a
	jr z, dst_label
	xor a
	ld (DV_LABEL), a
dst_label:
	ld de, DV_RIGHT
	xor a
	ld (de), a
	ld a, (DV_ST)
	or a
	jr nz, dst_cut
	ld hl, dist_block
	call app_strcpy
	dec de
	ld hl, (DV_NPAIRS)
	call app_dec16
	xor a
	ld (de), a
dst_cut:
	ld hl, DV_RIGHT
	call list_room
	ld hl, DV_LABEL
	ld de, DV_CUT
	ld b, LIST_LABEL_MAX + 2
	call far_ui_truncate
	ld hl, DV_CUT
	ld de, DV_RIGHT
	jp far_ui_title

; Describes the assertion DV_ASSERT: DV_NPAIRS, DV_PTR, the comma's glyph code and its width DV_WC.
; Out: A = the reader's status, NZ when not 0.
dist_open:
	ld a, UI_FONT_CH_2C
	ld (DV_COMMA), a
	xor a
	ld (DV_COMMA + 1), a
	ld hl, (DV_ASSERT)
	call far_lib_record
	or a
	ret nz
	ld hl, (lr_dnpairs)
	ld (DV_NPAIRS), hl
	ld hl, (lr_dpairs)
	ld (DV_PTR), hl
	ld a, (lr_dpairs + 2)
	ld (DV_PTR + 2), a
	ld hl, DV_COMMA
	call far_ui_text_width
	ld (DV_WC), hl
	xor a
	ret

; Places every pair as mcglyph's place does, a pair being its two tokens, a comma and 2 columns wide, 3
; columns after the one before it on a row. With DV_MODE 0 it draws nothing and leaves the last row's number
; in DV_ROW, FFFFh for no pair. With DV_MODE 1 it draws the pairs of the rows DV_TOP to DV_TOP + 5 and stops
; at the first pair of the next. Out: A = the reader's status, NZ when not 0.
dist_walk:
	ld hl, (DV_NPAIRS)
	ld (DV_LEFT), hl
	ld hl, 0FFFFh
	ld (DV_ROW), hl
	xor a
	ld (DV_BUFN), a
dsw_next:
	ld hl, (DV_LEFT)
	ld a, (DV_BUFN)
	or h
	or l
	jr z, dsw_done
	call dist_next
	ret nz
	ld a, (DV_IDA)
	call dist_token
	ret nz
	ld (DV_WA), hl
	ld a, (DV_IDB)
	call dist_token
	ret nz
	ld de, (DV_WA)
	add hl, de
	ld de, (DV_WC)
	add hl, de
	inc hl
	inc hl
	ex de, hl
	ld hl, (DV_ROW)
	inc hl
	ld a, h
	or l
	jr z, dsw_new
	ld hl, (DV_LW)
	inc hl
	inc hl
	inc hl
	ld (DV_X), hl
	add hl, de
	ld bc, DIST_WIDTH + 1
	or a
	sbc hl, bc
	jr nc, dsw_new
	ld hl, (DV_X)
	add hl, de
	ld (DV_LW), hl
	jr dsw_placed
dsw_new:
	ld hl, (DV_ROW)
	inc hl
	ld (DV_ROW), hl
	ld hl, 0
	ld (DV_X), hl
	ld (DV_LW), de
dsw_placed:
	ld a, (DV_MODE)
	or a
	jp z, dsw_next
	ld hl, (DV_ROW)
	ld de, (DV_TOP)
	or a
	sbc hl, de
	jp c, dsw_next
	ld a, h
	or a
	jr nz, dsw_done
	ld a, l
	cp DIST_ROWS
	jr nc, dsw_done
	inc a
	ld (DV_TR), a
	call dist_ink
	ret nz
	jp dsw_next
dsw_done:
	xor a
	ret

; Out: DV_IDA and DV_IDB = the next pair of the record, read DIST_RUN at a time into DV_BUF; A = the
; reader's status, NZ when not 0.
dist_next:
	ld a, (DV_BUFN)
	or a
	jr nz, dsn_take
	ld hl, (DV_LEFT)
	ld bc, DIST_RUN
	or a
	sbc hl, bc
	jr nc, dsn_run
	ld bc, (DV_LEFT)
	ld hl, 0
dsn_run:
	ld (DV_LEFT), hl
	ld a, c
	ld (DV_BUFN), a
	add a, a
	ld c, a
	ld b, 0
	push bc
	ld de, DV_BUF
	ld hl, (DV_PTR)
	ld a, (DV_PTR + 2)
	call far_lib_read
	pop bc
	or a
	ret nz
	ld hl, (DV_PTR)
	ld a, (DV_PTR + 2)
	add hl, bc
	adc a, 0
	ld (DV_PTR), hl
	ld (DV_PTR + 2), a
	ld hl, DV_BUF
	ld (DV_BUFP), hl
dsn_take:
	ld hl, (DV_BUFP)
	ld a, (hl)
	ld (DV_IDA), a
	inc hl
	ld a, (hl)
	ld (DV_IDB), a
	inc hl
	ld (DV_BUFP), hl
	ld hl, DV_BUFN
	dec (hl)
	xor a
	ret

; In: A = a symbol. Out: FM_SCRATCH = its glyph codes ended by 0, HL = their width in pixels, A = the
; reader's status, NZ when not 0, with HL undefined.
dist_token:
	ld de, FM_SCRATCH
	call far_lib_glyph
	or a
	ret nz
	ld hl, FM_SCRATCH
	ld c, b
	ld b, 0
	add hl, bc
	ld (hl), 0
	ld hl, FM_SCRATCH
	call far_ui_text_width
	xor a
	ret

; Draws the pair DV_IDA, DV_IDB at x DV_X on text row DV_TR: the first token, the comma one column on and the
; second token one column after it. Out: A = the reader's status, NZ when not 0.
dist_ink:
	ld a, (DV_IDA)
	call dist_token
	ret nz
	ld hl, FM_SCRATCH
	ld de, (DV_X)
	inc de
	ld a, (DV_TR)
	ld b, a
	call far_ui_text
	inc de
	ld hl, DV_COMMA
	ld a, (DV_TR)
	ld b, a
	call far_ui_text
	inc de
	ld (DV_X), de
	ld a, (DV_IDB)
	call dist_token
	ret nz
	ld hl, FM_SCRATCH
	ld de, (DV_X)
	ld a, (DV_TR)
	ld b, a
	call far_ui_text
	xor a
	ret
dist_end:

#else
dist_enter:
	xor a
	ret
dist_draw:
	xor a
	ret
dist_key:
	xor a
	ret
#endif
