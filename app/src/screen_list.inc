; Kind 6, the theorem list as List in crates/mcui/src/screens/list.rs draws it with every assertion of
; the stage shown (Kind::All, no find): the stage's name and checked/total in the title bar, five
; rows of labels with their marks, the cursor's row inverted, a one line preview of the cursor's
; conclusion on row 6, and the keys Open, Find, Kind, Try and Sect. The rows are the assertions 0 up
; to the stage's end, so the model's item index is the assertion's number and the screen keeps
; numbers and no item table. A stage with no assertion shows "none".
;
; Frame: byte 1 the stage, bytes 2 and 3 the cursor's assertion, 4 and 5 the first assertion shown,
; little endian. Position only: list_enter derives the rest in APP_TEXT and nothing in the frame says
; whether the preview is computed, so the enter that runs when a notice above the list pops, or a
; refresh after an auto power down, leaves the cursor and the top where they were and makes the
; preview pending again. The first push and an uncovering run the same enter.
;
; Derived state, in APP_TEXT:
;   LV_TABLE    1 when the status table is this library's (store_has), else 0; with 0 every row is
;               unchecked and the screen never creates a table
;   LV_FLAT     1 when the library has no trees section and an assertion: the model's "flat" prefix
;               on the counts, as Lib::degraded decides it, since a library whose database has no
;               grammar is packed with no trees
;   LV_SECT     1 when a section title starts inside the stage: the key F5 is live and its label drawn
;   LV_COUNT    the ticks the row must still be still, from LIST_WAIT, 11 (about 102 ms)
;   LV_PREVIEW  LVP_PENDING, the row is blank; LVP_ROW, fm holds the laid out conclusion of the cursor's
;               assertion and the draw shows it; LVP_NONE, settled with nothing to show
; The laid out row is fm's: fm_statement, fm_fit and fm_clip leave the statement, folded and cut to one
; line, in fm's RAM (FM_RAM, FM_LOW), and a settled draw calls fm_draw on it without fitting again.
; Only a kernel entry closes fm or overwrites that RAM, and none runs while a list is the top screen: the
; check job runs on the screen Libraries pushes, never over a list, and a list that is entered after it
; finds fm closed, which the preview's first fit answers with FM_ERR_NOT_OPEN and fm_open. A notice over
; the list, the screen's own refresh and the OS's overwrite of saveSScreen touch none of fm's memory,
; and every one of them runs list_enter again, which makes the preview pending, so a settled draw never
; follows a lost row.
;
; list_enter   in: HL = the frame. out: A = the status of libs_counts, which derives the counts the title
;              reads, or of the reader's assertion count; else 0. Reads the status table once (store_has),
;              the reader for the trees and the first section title, sets the preview pending and LV_COUNT
;              to LIST_WAIT.
; list_key     UP, DOWN, LEFT and RIGHT move the cursor one row or five, never past either end, move the
;              first row shown as little as it must to keep the cursor on screen, answer SCR_REDRAW with
;              the preview pending and LV_COUNT at LIST_WAIT, and answer SCR_STAY when nothing moved,
;              touching nothing. ENTER, F1, F2, F3 and F4, and F5 when LV_SECT is 1, push the notice
;              NOTICE_LIST of the stage. CLEAR pops. Every other key stays.
; list_idle    in: HL = the frame. out: A and carry as scr_idle takes them. LV_COUNT above 0: one app_tick,
;              one off the count, SCR_STAY with carry set. Count 0 and the preview pending: fm_statement of
;              the cursor's conclusion (after fm_open when fm answers FM_ERR_NOT_OPEN), fm_fit to one line
;              at LIST_WIDTH pixels, fm_clip to one line when it took more, then LVP_ROW and SCR_REDRAW with
;              carry clear. An empty stage and any fm status other than 0, which is what the model's
;              statement_within says for a statement past its caps (FM_ERR_LIMIT), settle as LVP_NONE and
;              answer SCR_STAY with carry clear. Settled: SCR_STAY with carry clear and nothing touched.
; list_draw    in: HL = the frame. Clears the canvas, stages list_block, draws the title, the rows, the
;              preview when LV_PREVIEW is LVP_ROW and the keys. It reads the mark of each row with one
;              store_get, five at most for a frame, and the label with one lib_label.
; Interrupts and speed: at every return of list_enter, list_key, list_idle and list_draw both are as at the
; call. fm's sessions and the reader's reads turn interrupts off for their own length and give the caller's
; state back; list_idle runs at the shell's speed, writes no port 20h and executes no DI or EI. app_tick
; is entered with interrupts on and returns with them on.

LIST_ROWS	.equ 5
LIST_WAIT	.equ 11
LIST_WIDTH	.equ 94
LIST_PREVIEW_ROW	.equ 6
LIST_NAME_SIZE	.equ 13
LIST_RIGHT_SIZE	.equ 22
LVP_PENDING	.equ 0
LVP_ROW		.equ 1
LVP_NONE		.equ 2
LIST_FM_NOT_OPEN	.equ 1
LIST_LIB_NO_TREES	.equ 9

LV_TABLE	.equ APP_TEXT
LV_FLAT		.equ APP_TEXT + 1
LV_SECT		.equ APP_TEXT + 2
LV_COUNT	.equ APP_TEXT + 3
LV_PREVIEW	.equ APP_TEXT + 4
LV_STAGE	.equ APP_TEXT + 5
LV_ROW		.equ APP_TEXT + 6
LV_CUR		.equ APP_TEXT + 7
LV_TOP		.equ APP_TEXT + 9
LV_END		.equ APP_TEXT + 11
LV_AT		.equ APP_TEXT + 13
LV_MARK		.equ APP_TEXT + 15
LV_RIGHT	.equ APP_TEXT + 17
LV_NAME		.equ LV_RIGHT + LIST_RIGHT_SIZE
LV_CUT		.equ LV_NAME + LIST_NAME_SIZE
LV_LABEL	.equ LV_CUT + LIST_NAME_SIZE
LV_TEXT		.equ LV_LABEL + APP_LABEL_MAX + 1
LV_VARS_END	.equ LV_TEXT + APP_LABEL_MAX + 2
#if LV_VARS_END > (APP_TEXT + APP_TEXT_SIZE)
 .error "the list screen's variables are longer than APP_TEXT"
#endif

#ifdef LIBRARY
; In: HL = the frame.
list_enter:
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
	xor a
	ret

; In: A = a stage. Out: DE = its end, the number of its assertions. Destroys AF HL.
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

; In: A = the key, HL = the frame.
list_key:
	cp KEY_UP
	jr z, lk_move
	cp KEY_DOWN
	jr z, lk_move
	cp KEY_LEFT
	jr z, lk_move
	cp KEY_RIGHT
	jr z, lk_move
	cp KEY_CLEAR
	jr z, lk_pop
	cp KEY_ENTER
	jr z, lk_notice
	cp KEY_F1
	jr c, lk_stay
	cp KEY_F5
	jr c, lk_notice
	jr nz, lk_stay
	ld a, (LV_SECT)
	or a
	jr z, lk_stay
lk_notice:
	inc hl
	ld c, (hl)
	ld b, NOTICE_LIST
	ld a, KIND_NOTICE
	jp screen_push_param
lk_pop:
	ld a, SCR_POP
	ret
lk_stay:
	ld a, SCR_STAY
	ret

lk_move:
	ld c, a
	push hl
	pop ix
	ld a, (ix + 1)
	call list_stage_end
	ld a, d
	or e
	jr z, lk_stay
	dec de
	ld l, (ix + 2)
	ld h, (ix + 3)
	ld a, c
	cp KEY_UP
	jr z, lk_up
	cp KEY_LEFT
	jr z, lk_left
	ld bc, 1
	cp KEY_DOWN
	jr z, lk_forward
	ld c, LIST_ROWS
lk_forward:
	add hl, bc
	or a
	sbc hl, de
	jr c, lk_inside
	ex de, hl
	jr lk_moved
lk_inside:
	add hl, de
	jr lk_moved
lk_up:
	ld bc, 1
	jr lk_back
lk_left:
	ld bc, LIST_ROWS
lk_back:
	or a
	sbc hl, bc
	jr nc, lk_moved
	ld hl, 0
lk_moved:
	ld a, (ix + 2)
	cp l
	jr nz, lk_set
	ld a, (ix + 3)
	cp h
	jr z, lk_stay
lk_set:
	ld (ix + 2), l
	ld (ix + 3), h
	ld e, (ix + 4)
	ld d, (ix + 5)
	or a
	sbc hl, de
	jr c, lk_above
	ld a, h
	or a
	jr nz, lk_below
	ld a, l
	cp LIST_ROWS
	jr c, lk_blank
lk_below:
	add hl, de
	ld de, 1 - LIST_ROWS
	add hl, de
	jr lk_top
lk_above:
	add hl, de
lk_top:
	ld (ix + 4), l
	ld (ix + 5), h
lk_blank:
	ld a, LVP_PENDING
	ld (LV_PREVIEW), a
	ld a, LIST_WAIT
	ld (LV_COUNT), a
	ld a, SCR_REDRAW
	ret

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
	push hl
	pop ix
	ld a, (ix + 1)
	call list_stage_end
	ld a, d
	or e
	jr z, li_none
	ld l, (ix + 2)
	ld h, (ix + 3)
	ld (LV_CUR), hl
	ld e, 0FFh
	call far_fm_statement
	cp LIST_FM_NOT_OPEN
	jr nz, li_loaded
	call far_fm_open
	or a
	jr nz, li_none
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
