; Kind 7, the theorem screen as Theorem in crates/mcui/src/screens/theorem.rs draws it: the label with the
; mark and "thm" or "ax" in the title bar, the hypotheses numbered and the conclusion laid out in six
; rows, folded by subexpression until they fit, a cursor that moves by subexpression, and the keys Fold,
; Dist, Why, Try and a blank. Dist pushes the Dist screen of the assertion (screen_dist.inc), Why and Try
; the stage's "Not available yet" notice. An assertion past the caps is the model's too large screen: the
; sentence and the keys Dist and Try. screen_theorem_draw.inc has the draw.
;
; Frame: byte 1 the stage, bytes 2 and 3 the assertion, as the list's cursor holds it, byte 4 the
; cursor's statement as the model counts them (the hypotheses from 0, the conclusion last), byte 5 its
; node, bytes 6 and 7 the first row shown, byte 8 "following", 1 exactly when the first row shown is the
; one the model's followed() gives for the cursor, byte 9 1 once an enter has run. Position only: the
; enter that runs when a notice above pops, or a refresh after an auto power down, is the model's refit.
; The scroll is two bytes because an assertion inside the caps can take more than 255 rows. Each token takes
; a row at most, so the bound is the 768 tokens of the caps; no shipped library comes near it: the largest
; total of rows is 7 in prop, 7 in pred, 13 in ring and 12 in the test fixture.
;
; Derived state, in APP_TEXT, rebuilt by every enter:
;   TV_TABLE    1 when the status table is this library's (store_has), else 0: the mark is unchecked
;   TV_LARGE    1 when fm refused the assertion or its fit with FM_ERR_LIMIT: nothing is laid out
;   TV_NHYPS    the hypotheses, so the statements 0 to TV_NHYPS, the last being the conclusion
;   TV_INDENT   the pixels a hypothesis is indented: the width of "N." for N hypotheses and 2, or 0
;   TV_TOTAL    the rows of all the statements together
;   TV_STARTS   the first row of each statement, two bytes each, and after the last the total
;   TV_CUT      the title's label cut to the room the mark and the kind leave
;   TV_RIGHT    the title's mark, a space and "thm" or "ax"
; The title is built once, by the enter, and drawn from there: no mark changes while a theorem is the top
; screen, because the only code the app runs that writes the status table is the check job (store_open,
; store_set and store_set_next), which runs on the checking screen that the Libraries screen pushes, and
; no key of the list or of this screen pushes that one. store_reset writes it too, and nothing in the app
; calls it.
; The statements, their folds and their line counts are fm's: fm_assertion and fm_fit leave them in fm's
; RAM, and every statement is selected before it is read or drawn. Only a kernel entry closes fm or
; overwrites that RAM and none runs while a theorem screen is open or covered: the check job runs
; on the screen Libraries pushes, and the list's idle work, which loads a statement with fm_statement,
; runs on the list. Every way the screen is entered after anything else ran, a notice above it popping
; or the refresh after an auto power down, loads the assertion and fits it again, so a draw
; never follows a lost layout. What writes APP_TEXT while the screen is open or covered: a notice above it
; (the notice's title and its own variables) and the OS's overwrite of saveSScreen at an auto power down,
; which takes APP_TEXT and the canvas's buffers and touches fm's RAM not at all; the enter builds all of it
; again. The rest of APP_TEXT is scratch that no call reads before it writes. A key copies bytes 4 to 8 of
; the frame to TV_S, TV_N, TV_SCROLL and TV_FOLLOWING and works on the copy. It writes the frame only
; when it ends well; a key that fails with the error notice leaves the frame untouched, though fm may
; hold a fold the key had already flipped and the starts may have been moved, and the enter that runs when
; the notice pops loads the assertion and fits it again.
;
; The draw, when fm refuses a call inside it, has no way to answer: a statement whose fm_select is refused
; is not drawn, no number, rows or marker, and the others are drawn as usual; a refused fm_cursor leaves
; the cursor's statement drawn with no rectangles; the status of fm_draw is not read. The next key or
; enter that fm refuses pushes the notice.
;
; Registers: each routine below may destroy AF BC DE HL and IX.
; theorem_enter  in: HL = the frame. Asks the store whether the table is the library's, builds the title,
;                loads the assertion with fm_assertion (after fm_open when fm answers FM_ERR_NOT_OPEN,
;                once) and fits it to six rows with fm_fit at the widths the model gives, a hypothesis
;                STATEMENT_WIDTH less the indent and the conclusion STATEMENT_WIDTH, then derives the rows.
;                FM_ERR_LIMIT from either is the too large screen and returns 0. Any other status fm
;                answers is returned, and so is the reader's from the assertion count and the read of the
;                CRC; store_has answering that the table is not this library's is no error, it clears
;                TV_TABLE, and a reader refusal while the title is built is none either: list_label_text
;                failing gives an empty label, far_lib_record failing gives "ax" and store_get failing
;                gives the unchecked mark.
;                The first enter, with byte 9 of the frame 0, puts the cursor on the conclusion's node 0
;                and the view where follow puts it. Later ones keep the cursor, clamped to the statements
;                and to the nodes of its statement, and follow it when the frame says the view was
;                following, else clamp the scroll. Then they write byte 8.
; theorem_key    UP, DOWN, LEFT, RIGHT, ENTER and F1 as the model's press; F2 pushes the Dist screen, F3 and F4
;                the notice NOTICE_LIST; CLEAR pops. F5, the letters and KEY_NONE are the model's no-op press: the frame is
;                left as it is when the view is following, else the view follows the cursor. On the too
;                large screen only F2, F4 and CLEAR act. Answers SCR_REDRAW exactly when the picture changes:
;                the statement, the scroll, a fold of a node that is not hidden, or the node drawn as the
;                cursor, which is the node itself or, for a node hidden inside a fold, its outermost
;                folded ancestor. Up from a node under a fold to the folded node stays, as both are drawn as
;                its ellipsis, but not to node 0, which is drawn as the marker with no rectangle. Else
;                SCR_STAY by one of two exits. tk_stay writes nothing to the frame: it answers F5, a letter
;                and KEY_NONE while byte 8 is 1 at once, calling nothing, a key that moved nothing while
;                byte 8 is 1, and the keys the too large screen ignores. tk_stayed writes bytes 4 to 8 of
;                the frame, the statement, node, scroll and following byte, for a key that moved the node
;                but not the node drawn as the cursor, and for a key that moved nothing while byte 8 is 0
;                and found the scroll where follow puts it. An arrow or Enter that moves nothing selects its
;                statement once and calls fm_step, fm_folded, fm_nodes or fm_fold as its path does, and
;                while byte 8 is 1 asks for no span. A status from fm pushes the error notice.
; theorem_push   in: HL = a list's frame. Fills scr_new with the theorem frame for the list's cursor, from
;                list_key. Out: A = SCR_PUSH.
; Interrupts and speed: every routine returns with both as at the call, and the screen writes no port 20h and
; executes no DI. fm's sessions and the reader's reads turn interrupts off for their own length and give the
; caller's state back, so the longest stretch with interrupts off is one session: 1,282,260 T-states in an
; open, the fit of ring's rglcom4d, and 79,906 in a move. The fit runs in the enter, so a tap
; inside it is lost.
; Stack, in bytes written below the stack pointer at the entry, the caller's return address counted, with
; interrupts masked, the same on both OSes unless said: list_key 8 for ENTER and F1; theorem_enter 24 on the
; stub OS and 34 on the reference OS, whose lookup of the table is deeper, for a push and for an uncovering,
; with a status table installed and without; theorem_draw 18, with the marker and with the cursor's
; rectangles, whether a statement is skipped or not. theorem_key by path: CLEAR 2; F2 8; F3 and F4 6; F5, a letter
; and KEY_NONE 4 while byte 8 is 1 and 12 while it is 0; Left and Right 14 to a sibling, 12 across
; statements, 10 where there is no statement to go to; Up 12 to node 0 and 14 to a parent that is not node
; 0, 10 at the root of a statement of several nodes; Down 14 to a first child and 10 at a leaf or at a
; folded node; on a node hidden inside a fold Left 10, Right 14, Up 14, Down 10, Enter and F1 10; Enter and
; F1 14 on a node that is not folded and on a folded one, which recount the statement, and 10 on a leaf; in
; the scroll case of a statement of one node Down and Up 12 with a row to scroll to, and with none 10 while
; byte 8 is 1 and 12 while it is 0. An interrupt that lands in a call adds the handler's bytes.
; Cost, in T-states at 6 MHz on the reference OS: the key that opens an assertion, the enter and the first frame
; take a median of 328,174 and at most 765,442 over prop's 779 assertions, and 297,537 and at most 1,785,376
; (297.6 ms, rglcom4d) over ring's 3,326, 6 of them over 200 ms and none over 300 ms, rglcom4d being the key
; 926, the enter 1,517,074 and the first frame 267,376; a cursor move with its frame, over every node of ring's
; 50 largest assertions and of every 97th of the rest, takes 270,149 at the median and 341,006 at most
; (df-rng, Down to node 63), 265 of 4,872 over 300,000, the worst being the key 56,666, the draw 209,786 and
; the blit 74,554; a fold with its frame 297,166 at the median and 394,252 at most (df-rng, Enter at node 68),
; 137 of 322 over, the worst being the key 109,835, the draw 209,863 and the blit 74,554.

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
TV_CUT		.equ TV_STARTS + (2 * (THM_STMTS + 1))
TV_RIGHT	.equ TV_CUT + LIST_LABEL_MAX + 2
TV_VARS_END	.equ TV_RIGHT + 8
#if TV_VARS_END > (APP_TEXT + APP_TEXT_SIZE)
 .error "the theorem screen's variables are longer than APP_TEXT"
#endif
TV_LABEL	.equ FM_SCRATCH + 64
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
	call theorem_title
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
	ld a, KIND_THEOREM
; In: A = a kind, HL = a frame with the stage in byte 1 and the assertion in bytes 2 and 3. Out: A = SCR_PUSH
; after filling scr_new with that kind's frame, the stage and the assertion set. Destroys AF BC DE HL IX.
theorem_push_kind:
	push hl
	pop ix
	ld b, (ix + 1)
	ld c, (ix + 2)
	call screen_push_param
	ld a, (ix + 3)
	ld (scr_new + 3), a
	ld a, SCR_PUSH
	ret

#else
theorem_enter:
	xor a
	ret
#endif
