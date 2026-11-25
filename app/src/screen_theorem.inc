; Kind 7, the theorem screen as Theorem in crates/mcui/src/screens/theorem.rs draws it: the label with the
; mark and "thm" or "ax" in the title bar, the hypotheses numbered and the conclusion laid out in six
; rows, folded by subexpression until they fit, a cursor that moves by subexpression, and the keys Fold,
; Dist, Why, Try and a blank. Dist, Why and Try push the stage's "Not available yet" notice. An
; assertion past the caps is the model's too large screen: the sentence and the keys Dist and Try.
; screen_theorem_draw.inc has the draw.
;
; Frame: byte 1 the stage, bytes 2 and 3 the assertion, as the list's cursor holds it, byte 4 the
; cursor's statement as the model counts them (the hypotheses from 0, the conclusion last), byte 5 its
; node, bytes 6 and 7 the first row shown, byte 8 "following", 1 exactly when the first row shown is the
; one the model's followed() gives for the cursor, byte 9 1 once an enter has run. Position only: the
; enter that runs when a notice above pops, or a refresh after an auto power down, is the model's refit.
; The scroll is two bytes because an assertion inside the caps can take more than 255 rows: a token
; wider than half the width sits alone on its line, a statement has at most 255 tokens and an
; assertion at most 768, and a statement fm cannot fold takes a line for each.
;
; Derived state, in APP_TEXT, rebuilt by every enter:
;   TV_TABLE    1 when the status table is this library's (store_has), else 0: the mark is unchecked
;   TV_LARGE    1 when fm refused the assertion or its fit with FM_ERR_LIMIT: nothing is laid out
;   TV_NHYPS    the hypotheses, so the statements 0 to TV_NHYPS, the last being the conclusion
;   TV_INDENT   the pixels a hypothesis is indented: the width of "N." for N hypotheses and 2, or 0
;   TV_TOTAL    the rows of all the statements together
;   TV_STARTS   the first row of each statement, two bytes each
; The statements, their folds and their line counts are fm's: fm_assertion and fm_fit leave them in fm's
; RAM, and every statement is selected before it is read or drawn. Only a kernel entry closes fm or
; overwrites that RAM and none runs while a theorem screen is open or covered: the check job runs
; on the screen Libraries pushes, and the list's idle work, which loads a statement with fm_statement,
; runs on the list. Every way the screen is entered after anything else ran, a notice above it popping
; or the refresh after an auto power down, loads the assertion and fits it again, so a draw
; never follows a lost layout. The OS's overwrite of saveSScreen touches fm's RAM not at all but
; takes APP_TEXT, which the enter builds again.
; The rest of APP_TEXT is scratch that no call reads before it writes. A key copies bytes 4 to 8 of the
; frame to TV_S, TV_N, TV_SCROLL and TV_FOLLOWING, works on the copy and writes it back, so a key that
; fails with the error notice changes nothing.
;
; Registers: each routine below may destroy AF BC DE HL and IX.
; theorem_enter  in: HL = the frame. Asks the store whether the table is the library's, loads the
;                assertion with fm_assertion (after fm_open when fm answers FM_ERR_NOT_OPEN, once) and
;                fits it to six rows with fm_fit at the widths the model gives, a hypothesis STATEMENT_WIDTH
;                less the indent and the conclusion STATEMENT_WIDTH, then derives the rows. FM_ERR_LIMIT from
;                either is the too large screen and returns 0; any other status from fm, the reader or
;                the store is returned. The first enter, with byte 9 of the frame 0, puts the cursor on the
;                conclusion's node 0 and the view where follow puts it. Later ones keep the cursor,
;                clamped to the statements and to the nodes of its statement, and follow it when the
;                frame says the view was following, else clamp the scroll. Then they write byte 8.
; theorem_key    UP, DOWN, LEFT, RIGHT, ENTER and F1 as the model's press; F2, F3 and F4 push the notice
;                NOTICE_LIST; CLEAR pops; every other key, F5 and the letters, stays. On the too large
;                screen only F2, F4 and CLEAR act. Answers SCR_REDRAW when the cursor, the scroll or a
;                fold changed, else SCR_STAY: a move between nodes a fold hides, or a fold of one, changes the
;                model's state and not its frame, and is answered with SCR_REDRAW and the same frame. A status
;                from fm pushes the error notice.
; theorem_push   in: HL = a list's frame. Fills scr_new with the theorem frame for the list's cursor, from
;                list_key. Out: A = SCR_PUSH.
; Interrupts and speed: every routine returns with both as at the call, and the screen writes no port 20h and
; executes no DI. fm's sessions and the reader's reads turn interrupts off for their own length and give the
; caller's state back, so the longest stretch with interrupts off is one session: 1,282,260 T-states in an open,
; the fit of ring's rglcom4d, and 78,040 in a key, a blit. The fit runs in the enter, so a tap inside it is lost.
; Stack, in bytes written below the stack pointer at the entry, the caller's return address counted, with
; interrupts masked: list_key 8 for ENTER and F1; theorem_enter 20 on the stub OS and 34 on the reference OS,
; whose lookup of the table is deeper, for a push and for an uncovering; theorem_key 14 for a move or a fold,
; 6 for F2 to F4, 2 for CLEAR, F5 and a letter; theorem_draw 26. An interrupt that lands in a call adds the
; handler's bytes.
; Cost, in T-states at 6 MHz on the reference OS: the key that opens an assertion, the enter and the first frame
; take a median of 332,318 and at most 770,485 over prop's 779 assertions, and 302,518 and at most 1,808,784
; (301.5 ms, rglcom4d) over ring's 3,326, 6 of them over 200 ms and one over 300 ms; a cursor move with its
; frame, over every node of ring's 50 largest assertions and of every 97th of the rest, takes 275,281 at the
; median and 337,104 at most (df-rng, Down to node 63), 520 of 4,872 over 300,000; a fold with its frame
; 321,753 and 391,103, 285 of 322 over.

THM_ROWS	.equ 6
THM_LARGE_ROW	.equ 2
THM_WIDTH	.equ 92
THM_GUTTER	.equ 3
THM_STMTS	.equ 32
THM_FM_NOT_OPEN	.equ 1
THM_FM_LIMIT	.equ 10

TV_S		.equ APP_TEXT
TV_N		.equ APP_TEXT + 1
TV_SCROLL	.equ APP_TEXT + 2
TV_FOLLOWING	.equ APP_TEXT + 4
TV_OLD		.equ APP_TEXT + 5
TV_ASSERT	.equ APP_TEXT + 9
TV_FRAME	.equ APP_TEXT + 11
TV_TABLE	.equ APP_TEXT + 13
TV_LARGE	.equ APP_TEXT + 14
TV_NHYPS	.equ APP_TEXT + 15
TV_INDENT	.equ APP_TEXT + 16
TV_TOTAL	.equ APP_TEXT + 17
TV_TOP		.equ APP_TEXT + 19
TV_BOT		.equ APP_TEXT + 21
TV_ROW		.equ APP_TEXT + 23
TV_I		.equ APP_TEXT + 25
TV_KEY		.equ APP_TEXT + 26
TV_DOFOLLOW	.equ APP_TEXT + 27
TV_CHG		.equ APP_TEXT + 28
TV_X		.equ APP_TEXT + 29
TV_STARTS	.equ APP_TEXT + 30
TV_VARS_END	.equ TV_STARTS + (2 * THM_STMTS)
#if TV_VARS_END > (APP_TEXT + APP_TEXT_SIZE)
 .error "the theorem screen's variables are longer than APP_TEXT"
#endif
TV_LABEL	.equ FM_SCRATCH + 64
TV_CUT		.equ FM_SCRATCH + 128
TV_RIGHT	.equ FM_SCRATCH + 192
TV_NUMBER	.equ FM_SCRATCH

#ifdef LIBRARY
; In: HL = the frame.
theorem_enter:
	ld (TV_FRAME), hl
	push hl
	pop ix
	ld l, (ix + 2)
	ld h, (ix + 3)
	ld (TV_ASSERT), hl
	xor a
	ld (TV_LARGE), a
	ld (TV_TABLE), a
	call far_lib_assertion_count
	ret nz
	push hl
	ld bc, 4
	ld de, APP_CRC
	ld hl, 4
	xor a
	call far_lib_read
	pop bc
	or a
	ret nz
	ld hl, (APP_CRC)
	ld de, (APP_CRC + 2)
	call store_has
	or a
	jr nz, te_load
	inc a
	ld (TV_TABLE), a
te_load:
	ld hl, (TV_ASSERT)
	call far_fm_assertion
	cp THM_FM_NOT_OPEN
	jr nz, te_loaded
	call far_fm_open
	or a
	ret nz
	ld hl, (TV_ASSERT)
	call far_fm_assertion
te_loaded:
	cp THM_FM_LIMIT
	jp z, te_large
	or a
	ret nz
	ld a, b
	dec a
	ld (TV_NHYPS), a
	call theorem_indent
	ld a, (TV_INDENT)
	ld e, a
	ld a, THM_WIDTH
	sub e
	ld d, a
	ld e, THM_WIDTH
	ld b, THM_ROWS
	call far_fm_fit
	cp THM_FM_LIMIT
	jp z, te_large
	or a
	ret nz
	call theorem_place
	ret nz
	ld ix, (TV_FRAME)
	ld a, (ix + 9)
	or a
	jr nz, te_again
	ld a, (TV_NHYPS)
	ld (TV_S), a
	xor a
	ld (TV_N), a
	ld h, a
	ld l, a
	ld (TV_SCROLL), hl
	inc a
	ld (TV_DOFOLLOW), a
	jr te_settle
te_again:
	push ix
	pop hl
	ld de, 4
	add hl, de
	ld de, TV_S
	ld bc, 5
	ldir
	ld a, (TV_NHYPS)
	ld hl, TV_S
	cp (hl)
	jr nc, te_stmt
	ld (hl), a
te_stmt:
	ld a, (TV_S)
	call theorem_select
	ret nz
	call far_fm_nodes
	or a
	ret nz
	dec b
	ld a, (TV_N)
	cp b
	jr c, te_node
	ld a, b
	ld (TV_N), a
te_node:
	ld a, (TV_FOLLOWING)
	ld (TV_DOFOLLOW), a
	or a
	jr nz, te_settle
	ld hl, (TV_SCROLL)
	call theorem_clamp
	ld (TV_SCROLL), hl
te_settle:
	call theorem_finish
	ret nz
	ld ix, (TV_FRAME)
	ld (ix + 9), 1
	jp theorem_store
te_large:
	ld a, 1
	ld (TV_LARGE), a
	ld ix, (TV_FRAME)
	ld (ix + 9), a
	xor a
	ret

; In: HL = the frame. Out: A = SCR_PUSH after filling scr_new.
theorem_push:
	push hl
	pop ix
	ld b, (ix + 1)
	ld c, (ix + 2)
	ld a, KIND_THEOREM
	call screen_push_param
	ld a, (ix + 3)
	ld (scr_new + 3), a
	ld a, SCR_PUSH
	ret

; Derives TV_INDENT from TV_NHYPS: the width of "N." and 2, 0 for no hypothesis.
theorem_indent:
	ld a, (TV_NHYPS)
	or a
	jr z, ti_set
	ld l, a
	ld h, 0
	ld de, TV_NUMBER
	call app_dec16
	ld a, UI_FONT_CH_2E
	ld (de), a
	inc de
	xor a
	ld (de), a
	ld hl, TV_NUMBER
	call far_ui_text_width
	ld a, l
	add a, 2
ti_set:
	ld (TV_INDENT), a
	ret

; In: A = a statement. Out: D = the pixels it is laid out at. Destroys AF E HL.
theorem_width:
	ld d, THM_WIDTH
	ld hl, TV_NHYPS
	cp (hl)
	ret z
	ld a, (TV_INDENT)
	ld e, a
	ld a, d
	sub e
	ld d, a
	ret

; In: A = a statement. Out: A = fm_select's status, NZ when not 0. Destroys all.
theorem_select:
	ld e, a
	ld hl, TV_NHYPS
	cp (hl)
	jr nz, ts_call
	ld e, 0FFh
ts_call:
	call far_fm_select
	or a
	ret

; Derives TV_STARTS and TV_TOTAL from the line counts fm holds, leaving the conclusion selected.
; Out: A = a status, NZ when not 0. Destroys all.
theorem_place:
	ld hl, 0
	ld (TV_ROW), hl
	xor a
	ld (TV_I), a
tp_next:
	ld a, (TV_I)
	call theorem_select
	ret nz
	ld a, (TV_I)
	call theorem_width
	call far_fm_rows
	or a
	ret nz
	ld a, (TV_I)
	add a, a
	ld e, a
	ld d, 0
	ld hl, TV_STARTS
	add hl, de
	ld de, (TV_ROW)
	ld (hl), e
	inc hl
	ld (hl), d
	ld l, b
	ld h, 0
	add hl, de
	ld (TV_ROW), hl
	ld hl, TV_I
	inc (hl)
	ld a, (TV_NHYPS)
	cp (hl)
	jr nc, tp_next
	ld hl, (TV_ROW)
	ld (TV_TOTAL), hl
	xor a
	ret

; In: HL = a first row. Out: HL = the first row, at most the rows less six, or 0. Destroys AF DE.
theorem_clamp:
	push hl
	ld hl, (TV_TOTAL)
	ld de, -THM_ROWS
	add hl, de
	jr c, tc_max
	ld hl, 0
tc_max:
	pop de
	push de
	push hl
	or a
	sbc hl, de
	pop hl
	pop de
	ret c
	ex de, hl
	ret

; Out: HL = the first row the model's followed() gives for the cursor and TV_SCROLL, A = a status, NZ when
; not 0. Destroys all.
theorem_followed:
	ld a, (TV_S)
	call theorem_select
	ret nz
	ld a, (TV_S)
	call theorem_width
	ld a, (TV_N)
	ld e, a
	call far_fm_span
	or a
	ret nz
	push bc
	ld a, (TV_S)
	add a, a
	ld e, a
	ld d, 0
	ld hl, TV_STARTS
	add hl, de
	ld e, (hl)
	inc hl
	ld d, (hl)
	pop bc
	ld l, b
	ld h, 0
	add hl, de
	ld (TV_TOP), hl
	ld l, c
	ld h, 0
	add hl, de
	ld (TV_BOT), hl
	ld de, (TV_SCROLL)
	ld hl, (TV_TOP)
	or a
	sbc hl, de
	ld hl, (TV_TOP)
	jr c, tf_clamp
	ld hl, THM_ROWS
	add hl, de
	ex de, hl
	ld hl, (TV_BOT)
	or a
	sbc hl, de
	ld hl, (TV_SCROLL)
	jr c, tf_clamp
	jr z, tf_clamp
	ld hl, (TV_BOT)
	ld de, -THM_ROWS
	add hl, de
	ld de, (TV_TOP)
	push hl
	or a
	sbc hl, de
	pop hl
	jr c, tf_clamp
	ex de, hl
tf_clamp:
	call theorem_clamp
	xor a
	ret

; Moves the view to the cursor when TV_DOFOLLOW says so, then sets TV_FOLLOWING from whether the view is
; where follow would put it. Out: A = a status, NZ when not 0. Destroys all.
theorem_finish:
	call theorem_followed
	ret nz
	ld a, (TV_DOFOLLOW)
	or a
	jr z, tn_keep
	ld (TV_SCROLL), hl
tn_keep:
	ld de, (TV_SCROLL)
	or a
	sbc hl, de
	ld a, 0
	jr nz, tn_set
	inc a
tn_set:
	ld (TV_FOLLOWING), a
	xor a
	ret

; Writes TV_S to TV_FOLLOWING into the frame's bytes 4 to 8. Out: A = 0. Destroys AF BC DE HL.
theorem_store:
	ld hl, (TV_FRAME)
	ld de, 4
	add hl, de
	ex de, hl
	ld hl, TV_S
	ld bc, 5
	ldir
	xor a
	ret

#else
theorem_enter:
	xor a
	ret
#endif
