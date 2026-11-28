; Kind 6, the theorem list as List in crates/mcui/src/screens/list.rs draws it with the assertions
; its kind shows (Kind::All, Theorems, Axioms or Unchecked, no find): the stage's name, then ":thm", ":ax"
; or ":?" for the last three, and checked/total in the title bar, five
; rows of labels with their marks, the cursor's row inverted, a one line preview of the cursor's
; conclusion on row 6, and the keys Open, Find, Kind, Try and Sect. The rows are the assertions 0 up
; to the stage's end that the kind shows, and the screen keeps their numbers and no item table, so the model's
; item index is the assertion's number only under All (screen_list_kind.inc finds the next and the previous
; assertion of a kind). A stage with no assertion, and a kind that shows none, show "none".
;
; Frame: byte 1 the stage, bytes 2 and 3 the cursor's assertion, 4 and 5 the first assertion shown,
; little endian, byte 6 the kind (0 All, 1 Theorems, 2 Axioms, 3 Unchecked). Both numbers are assertions the
; kind shows, and 0 and 0 when it shows none. Position only: list_enter derives the rest in APP_TEXT and
; nothing in the frame says whether the preview is computed, so the enter that runs when a notice above
; the list pops, or a refresh after an auto power down, makes the preview pending again and leaves the
; kind byte, the cursor and the top as they were, except that under a kind a cursor or a first row the
; kind does not show (the marks may have changed since the frame was made) is put back as F3 lands them,
; a cursor above the first row gets the first row on the cursor, and a cursor the new marks put more than
; four shown assertions below the first row gets the first row four shown assertions above it. The first
; push and an uncovering run the same enter.
;
; Derived state, in APP_TEXT:
;   LV_HASK     1 when the library has the kinds section, which F3 needs: with 0 there is only All
;   LV_KIND     the frame's kind byte, copied by each routine
;   LV_TABLE    1 when the status table is this library's (store_has), else 0; with 0 every row is
;               unchecked and the screen never creates a table. A table lost after the enter leaves it
;               at 1: the store then answers lf_next and lf_prev with a status other than 0, and they
;               answer as with no table, so Unchecked shows every assertion, and the title keeps
;               the counts list_enter derived
;   LV_FLAT     1 when the library has no trees section and an assertion: the model's "flat" prefix
;               on the counts. That is Lib::degraded for a library mcpack packs, which writes no
;               trees section exactly when the database has no grammar; a library with its trees
;               section removed by other means is flat here and not in the model
;   LV_SECT     1 when a section title starts inside the stage: the key F5 is live and its label drawn
;   LV_COUNT    the ticks the row must still be still, from LIST_WAIT
;   LV_PREVIEW  LVP_PENDING, the row is blank; LVP_ROW, fm holds the laid out conclusion of the cursor's
;               assertion and the draw shows it; LVP_NONE, settled with nothing to show
;   LV_NOFM     1 once fm_open has answered anything but 0 since list_enter, else 0: every preview
;               then settles as LVP_NONE without a call to fm
; The rest of APP_TEXT is scratch that no call reads before it writes: list_enter writes LV_STAGE,
; list_draw writes LV_STAGE, LV_CUR, LV_TOP, LV_END, LV_ROW, LV_AT, LV_MARK and the strings it builds,
; and list_idle writes LV_CUR. list_enter, list_key and list_idle also write LV_KIND and LV_END, and the
; LK_ scratch of screen_list_kind.inc at the start of LV_RIGHT, which list_draw builds again before it reads;
; LK_AGAIN holds the number lf_next and lf_prev were given while the store is asked.
; The laid out row is fm's: fm_statement, fm_fit and fm_clip leave the statement, folded and cut to one
; line, in fm's RAM (FM_RAM, FM_LOW), and a settled draw calls fm_draw on it without fitting again.
; Only a kernel entry closes fm or overwrites that RAM, and none runs while a list is the top screen: the
; check job runs on the screen Libraries pushes, never over a list, and a list that is entered after it
; finds fm closed, which the first fm_statement of its preview answers with FM_ERR_NOT_OPEN, and list_idle
; calls fm_open, once until the next list_enter if that fails. A notice over the list, the screen's own
; refresh and the OS's overwrite of saveSScreen touch none of fm's memory, and every one of them runs
; list_enter again, which makes the preview pending, so a settled draw never follows a lost row.
;
; Registers: each routine below may destroy AF BC DE HL and IX, which the far calls and the reader leave
; undefined, except list_stage_end, which says what it keeps.
; list_enter   in: HL = the frame. out: A = the status of libs_counts, which derives the counts the title
;              reads, or of the reader's assertion count; else 0. After the first status other than 0 it
;              returns with every variable as it was, and after the second with LV_COUNT, LV_PREVIEW
;              and LV_NOFM as they were and LV_TABLE, LV_FLAT, LV_SECT, LV_HASK and LV_KIND at 0.
;              Otherwise it asks the store (store_has) whether the table is the library's, the reader
;              whether it has trees and where the first section title starts, sets the preview pending
;              and LV_COUNT to LIST_WAIT, and calls list_enter_kinds, which asks the reader whether it
;              has the kinds section (LV_HASK) and, for a frame whose kind is not All, with LV_KIND and
;              LV_END set, puts the frame's places back on assertions the kind shows: lf_next for the
;              cursor and, only when the cursor is shown, lf_next for the first row; when either is not
;              the assertion it was given, list_kind_land rewrites bytes 2 to 5 of the frame, never
;              byte 6. When both are shown and differ, a cursor above the first row gets the first row
;              on the cursor, and otherwise the walk of list_move_kind goes from the first row: a cursor
;              among the five shown from it leaves the frame, any other gets the first row four shown
;              assertions above it, which happens when a table lost or reset since the frame was made
;              shows assertions between them. Last, LV_NOFM is set to 0.
; list_idle    in: HL = the frame. out: A and carry as scr_idle takes them. LV_COUNT above 0: one off the
;              count, one app_tick, SCR_STAY with carry set. Count 0 and the preview pending, with
;              LV_NOFM 0, which writes LK_FR under every kind, and a stage that has assertions, under a
;              kind: one lf_next, a lib_kind_next under Theorems and Axioms or a store_next_not_one
;              under Unchecked with a table (which writes LK_AGAIN), with LV_KIND and LV_END written;
;              when the cursor's assertion is not the one it answers, the preview settles as LVP_NONE
;              with SCR_STAY and carry clear and fm is not called. Otherwise, and under All: fm_statement of the cursor's conclusion (after
;              fm_open when fm answers FM_ERR_NOT_OPEN), fm_fit to one line at LIST_WIDTH pixels,
;              fm_clip to one line when it took more, then LVP_ROW and SCR_REDRAW with carry clear. An empty
;              stage settles as LVP_NONE with SCR_STAY and carry clear, calling nothing of fm. So does
;              LV_NOFM set, and an fm_open that answers anything but 0 sets it: a library fm cannot open costs one fm_open per list_enter, and every
;              other rest calls neither fm_open nor fm_statement. So does any status other than 0 from
;              fm_statement, fm_fit or fm_clip: FM_ERR_LIMIT is the model's "over the caps"
;              (statement_within), and any other (a refused read, a damaged tree) settles with nothing
;              too, where the model would draw the preview. Settled: SCR_STAY with carry clear and
;              nothing touched.
;              The count is in timer interrupts and the first halt ends at the next one, so the fit
;              starts between 10 and 11 periods after the first call that follows a key's frame:
;              578,942 T-states (96.5 ms) measured on the reference OS.
; list_draw    in: HL = the frame. Clears the canvas, stages list_block, draws the title, the rows, the
;              preview when LV_PREVIEW is LVP_ROW, by one fm_draw and no fit, and the keys. It reads the
;              label of each row with one lib_label and, when LV_TABLE is 1, its mark with one
;              store_get, five of each at most for a frame. Under a kind it also asks lf_next once
;              whether the first row is shown (when it is not the screen says none, as for an empty
;              stage) and once after each row drawn, six calls at most, and writes LV_KIND and LV_END.
; list_land   the land routine of the kind table, in screen_list_land.inc: after the sections screen's jump has
;              popped and this screen's enter has run, it clears the find in byte 7 of the frame and moves the cursor
;              and the first row to the section's first assertion that the kind shows. The preview stays pending
;              for the wait that list_enter set, also when the cursor did not move.
; Interrupts and speed: list_enter, list_key and list_draw return with both as at the call, and so does
; list_idle but for the tick path, which returns with interrupts on, as the shell calls it, because
; app_tick ends in ei. fm's sessions and the reader's reads turn interrupts off for their own length and
; give the caller's state back; the screen writes no port 20h and executes no DI.
; Stack, in bytes written below the stack pointer at the entry, the caller's return address counted, the
; greatest over a call of each routine with interrupts masked, at both speeds (list_depth.rs; the figures
; under a kind are in screen_list_kind.inc). Stub OS under All: list_key 4 for a move and for a key at an
; end, 6 for F2 and F4, which push the notice, 8 for ENTER and F1 on a stage with an assertion, which
; push the theorem screen (under a kind they ask lf_next first and write 16, 16 and 22 under Theorems,
; Axioms and Unchecked on the stub and 16, 16 and 36 on the reference OS), 2 for CLEAR, a letter and F5 with no sections (with sections it is sections_push's: 18 under All, Theorems and Axioms, and under Unchecked with a table 22 on the stub and 36 on the reference OS), F3 from All 20 on
; the fixture of list_depth.rs and 24 on the proofs library of list_kind_depth.rs, from the first
; assertion and from ten rows down alike; list_enter 22; list_idle 8 for the fit and for a tick, which holds
; app_tick's return address and the interrupt handler's pushes, and 2 settled; list_draw 26. Reference
; OS, prop with a table: the same but F3 from All 24, a tick 14, list_enter 36, list_draw 34. An interrupt
; that lands in a call can only add to these: the figures with the app's interrupts on are never below
; the masked ones.
; Cost, in T-states at 6 MHz on the reference OS, over ring (list_kind_real.rs, list_real_cost.rs). Under a
; kind a move is list_key then scr_draw and ui_blit: under Axioms a row from 298 to 567 takes 267,170 and a
; page from assertion 5 takes 296,075, or 289,499 with the first row on the cursor, the five matches
; crossing the 268 assertions after 298; under Theorems a row across the run at 2,988 takes 259,683 and a
; page 242,678. Under Unchecked on a table of ones with only the first and the last assertion not 1 the row
; move from 0 to the last takes 306,465 and back 304,707, both over the 300,000 of a move. F3 with its frame takes 259,343 from Theorems to
; Axioms with the cursor just after an axiom and 314,654 into Unchecked from assertion 1 on that table.
; Ring's list opens in 461,598 (libs_key, list_enter and a cold frame) and, under All, a cursor move takes
; 223,339 by a row and 227,921 by a page with its frame; the idle call that lays out a preview takes a
; median of 77,198 and at most 1,125,910 over ring, and 68,891 and 331,490 over prop.
; Its fm calls run with interrupts off and no key is scanned until it returns, so a statement inside the
; caps holds the keypad for at most 37,707,349 T-states (6.28 s at 6 MHz), the 253 nested operators of
; fm's nodes fixture, measured on the stub OS.

LIST_ROWS	.equ 5
LIST_LABEL_MAX	.equ 48
LIST_WAIT	.equ 11
LIST_WIDTH	.equ 94
LIST_PREVIEW_ROW	.equ 6
LIST_NAME_SIZE	.equ 13
LIST_CUT_SIZE	.equ LIST_NAME_SIZE + 5
LIST_RIGHT_SIZE	.equ 22
LVP_PENDING	.equ 0
LVP_ROW		.equ 1
LVP_NONE		.equ 2
LIST_FM_NOT_OPEN	.equ 1
LIST_LIB_NO_TREES	.equ 9
LIST_KS_SECTION	.equ 20h

LV_TABLE	.equ APP_TEXT
LV_FLAT		.equ APP_TEXT + 1
LV_SECT		.equ APP_TEXT + 2
LV_COUNT	.equ APP_TEXT + 3
LV_PREVIEW	.equ APP_TEXT + 4
LV_NOFM		.equ APP_TEXT + 5
LV_STAGE	.equ APP_TEXT + 6
LV_ROW		.equ APP_TEXT + 7
LV_CUR		.equ APP_TEXT + 8
LV_TOP		.equ APP_TEXT + 10
LV_END		.equ APP_TEXT + 12
LV_AT		.equ APP_TEXT + 14
LV_MARK		.equ APP_TEXT + 16
LV_HASK		.equ APP_TEXT + 18
LV_KIND		.equ APP_TEXT + 19
LV_RIGHT	.equ APP_TEXT + 20
LV_NAME		.equ LV_RIGHT + LIST_RIGHT_SIZE
LV_CUT		.equ LV_NAME + LIST_NAME_SIZE
LV_LABEL	.equ LV_CUT + LIST_CUT_SIZE
LV_TEXT		.equ LV_LABEL + LIST_LABEL_MAX + 1
LV_VARS_END	.equ LV_TEXT + LIST_LABEL_MAX + 2
LK_FR		.equ LV_RIGHT
LK_CUR		.equ LV_RIGHT + 2
LK_X		.equ LV_RIGHT + 4
LK_T		.equ LV_RIGHT + 6
LK_WAS		.equ LV_RIGHT + 8
LK_N		.equ LV_RIGHT + 10
LK_KEY		.equ LV_RIGHT + 11
LK_OLD		.equ LV_RIGHT + 12
LK_EMPTY	.equ LV_RIGHT + 13
LK_AGAIN		.equ LV_RIGHT + 14
LK_T1		.equ LV_RIGHT + 16
#if LV_VARS_END > (APP_TEXT + APP_TEXT_SIZE)
 .error "the list screen's variables are longer than APP_TEXT"
#endif
#if (LK_T1 + 2) > (LV_RIGHT + LIST_RIGHT_SIZE)
 .error "the list kind scratch runs past LV_RIGHT"
#endif

#ifdef LIBRARY
; In: HL = the frame.
list_enter:
	ld (LK_FR), hl
	push hl
	pop ix
	ld a, (ix + 1)
	ld (LV_STAGE), a
	call libs_counts
	or a
	ret nz
	ld (LV_TABLE), a
	ld (LV_FLAT), a
	ld (LV_SECT), a
	ld (LV_HASK), a
	ld (LV_KIND), a
	call far_lib_assertion_count
	ret nz
	ld a, h
	or l
	jr z, le_table
	push hl
	call far_lib_syntax_count
	cp LIST_LIB_NO_TREES
	jr nz, le_trees
	ld a, 1
	ld (LV_FLAT), a
le_trees:
	pop hl
le_table:
	ld b, h
	ld c, l
	ld hl, (APP_CRC)
	ld de, (APP_CRC + 2)
	call store_has
	or a
	jr nz, le_sections
	inc a
	ld (LV_TABLE), a
le_sections:
	call far_lib_title_count
	or a
	jr nz, le_wait
	ld hl, 0
	ld de, FM_SCRATCH
	call far_lib_title
	or a
	jr nz, le_wait
	push hl
	ld a, (LV_STAGE)
	call list_stage_end
	pop hl
	or a
	sbc hl, de
	jr nc, le_wait
	ld a, 1
	ld (LV_SECT), a
le_wait:
	ld a, LIST_WAIT
	ld (LV_COUNT), a
	ld a, LVP_PENDING
	ld (LV_PREVIEW), a
	call list_enter_kinds
	xor a
	ld (LV_NOFM), a
	ret

; In: A = a stage. Out: DE = its end, the number of its assertions. Destroys F HL.
list_stage_end:
	ld l, a
	ld h, 0
	add hl, hl
	ld de, APP_LIST + 1
	add hl, de
	ld e, (hl)
	inc hl
	ld d, (hl)
	ret

#include "screen_list_key.inc"

; In: HL = the frame.
list_idle:
	ld a, (LV_COUNT)
	or a
	jr z, li_work
	dec a
	ld (LV_COUNT), a
	call app_tick
	ld a, SCR_STAY
	scf
	ret
li_work:
	ld a, (LV_PREVIEW)
	or a
	jr z, li_compute
	xor a
	ret
li_compute:
	ld a, (LV_NOFM)
	or a
	jp nz, li_none
	ld (LK_FR), hl
	push hl
	pop ix
	ld a, (ix + 1)
	call list_stage_end
	ld a, d
	or e
	jr z, li_none
	ld a, (ix + 6)
	or a
	jr z, li_shown
	ld (LV_KIND), a
	ld (LV_END), de
	ld l, (ix + 2)
	ld h, (ix + 3)
	call lf_next
	ld ix, (LK_FR)
	ld e, (ix + 2)
	ld d, (ix + 3)
	or a
	sbc hl, de
	jr nz, li_none
li_shown:
	ld l, (ix + 2)
	ld h, (ix + 3)
	ld (LV_CUR), hl
	ld e, 0FFh
	call far_fm_statement
	cp LIST_FM_NOT_OPEN
	jr nz, li_loaded
	call far_fm_open
	or a
	jr z, li_opened
	ld a, 1
	ld (LV_NOFM), a
	jr li_none
li_opened:
	ld hl, (LV_CUR)
	ld e, 0FFh
	call far_fm_statement
li_loaded:
	or a
	jr nz, li_none
	ld b, 1
	ld d, LIST_WIDTH
	ld e, LIST_WIDTH
	call far_fm_fit
	or a
	jr nz, li_none
	dec hl
	ld a, h
	or l
	jr z, li_row
	ld b, 1
	ld d, LIST_WIDTH
	call far_fm_clip
	or a
	jr nz, li_none
li_row:
	ld a, LVP_ROW
	ld (LV_PREVIEW), a
	ld a, SCR_REDRAW
	or a
	ret
li_none:
	ld a, LVP_NONE
	ld (LV_PREVIEW), a
	xor a
	ret
#else
list_enter:
list_draw:
list_idle:
	xor a
	ret
list_key:
	xor a
	ret
#endif
