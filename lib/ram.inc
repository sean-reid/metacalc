#ifndef LIB_STATE
LIB_STATE           .equ $989C
#endif
#ifndef LIB_BOUNCE
LIB_BOUNCE          .equ $FE00
#endif
#ifndef LIB_SPARE_PAGE
LIB_SPARE_PAGE      .equ 3
#endif
#if LIB_SPARE_PAGE > 7
 .error "LIB_SPARE_PAGE must be a RAM page, 0 to 7"
#endif
#if LIB_BOUNCE < $C000
 .error "LIB_BOUNCE must be in bank C"
#endif
#if LIB_BOUNCE + $100 > $10000
 .error "LIB_BOUNCE must leave 256 bytes below $10000"
#endif
#ifndef LIB_BOUNCE_ANYWHERE
#if LIB_BOUNCE < $C500
 .error "LIB_BOUNCE must lie inside the kernel regions, $C500 to $D6FF or $DF00 to $FEFF"
#endif
#if LIB_BOUNCE + $100 > $FF00
 .error "LIB_BOUNCE must lie inside the kernel regions, $C500 to $D6FF or $DF00 to $FEFF"
#endif
#if LIB_BOUNCE + $100 > $D700
#if LIB_BOUNCE < $DF00
 .error "LIB_BOUNCE must lie inside the kernel regions, $C500 to $D6FF or $DF00 to $FEFF"
#endif
#endif
#endif
#if LIB_STATE < $8000
 .error "LIB_STATE must be in bank B"
#endif
LIB_STATE_SIZE      .equ $2D0
LIB_INDEX           .equ LIB_STATE+$A8
#if LIB_STATE + LIB_STATE_SIZE > $C000
 .error "LIB_STATE and its index must end below $C000"
#endif
OPEN_MAGIC          .equ $4C42

lr_dest             .equ LIB_STATE+$00
lr_off              .equ LIB_STATE+$02
lr_old5             .equ LIB_STATE+$05
lr_iff              .equ LIB_STATE+$06
lr_page1            .equ LIB_STATE+$07
lr_n2               .equ LIB_STATE+$08
lr_src              .equ LIB_STATE+$09
lr_n1               .equ LIB_STATE+$0B

lr_open             .equ LIB_STATE+$10
lr_page             .equ LIB_STATE+$12
lr_length           .equ LIB_STATE+$13
lr_consts           .equ LIB_STATE+$16
lr_nsyms            .equ LIB_STATE+$17
lr_count            .equ LIB_STATE+$18
lr_dir              .equ LIB_STATE+$1A

lr_dn               .equ LIB_STATE+$20
lr_drec             .equ LIB_STATE+$22
lr_dkind            .equ LIB_STATE+$26
lr_dllen            .equ LIB_STATE+$27
lr_dnhyps           .equ LIB_STATE+$28
lr_dhyps            .equ LIB_STATE+$29
lr_dnpairs          .equ LIB_STATE+$2C
lr_dpairs           .equ LIB_STATE+$2E
lr_dconc            .equ LIB_STATE+$31
lr_dclen            .equ LIB_STATE+$34
lr_hk               .equ LIB_STATE+$35
lr_hoff             .equ LIB_STATE+$36

lr_bufp             .equ LIB_STATE+$40
lr_scratch          .equ LIB_STATE+$42
lr_hdr              .equ LIB_STATE+$44
lr_tr               .equ LIB_STATE+$48
lr_sr               .equ LIB_STATE+$4C
lr_sid              .equ LIB_STATE+$4E
lr_sbase            .equ LIB_STATE+$4F
lr_want             .equ LIB_STATE+$50
lr_id               .equ LIB_STATE+$51
lr_ent              .equ LIB_STATE+$52
lr_len              .equ LIB_STATE+$55
lr_tc               .equ LIB_STATE+$56
lr_n                .equ LIB_STATE+$57
lr_k                .equ LIB_STATE+$59
lr_cnt              .equ LIB_STATE+$5A
lr_wbase            .equ LIB_STATE+$5B
lr_wlen             .equ LIB_STATE+$5E
lr_wp               .equ LIB_STATE+$5F
lr_win              .equ LIB_STATE+$60
LIB_WINDOW          .equ 32
lr_dcbase           .equ LIB_STATE+$80
lr_q                .equ LIB_STATE+$82
lr_dc               .equ LIB_STATE+$84
lr_cn               .equ LIB_STATE+$A4

ls_have             .equ LIB_STATE+$2A8
ls_ti               .equ LIB_STATE+$2A9
ls_gl               .equ LIB_STATE+$2B2
ls_tr               .equ LIB_STATE+$2BB
ls_ti_n             .equ LIB_STATE+$2C4
ls_gl_fp            .equ LIB_STATE+$2C6
ls_tr_n             .equ LIB_STATE+$2C8
ls_ts               .equ LIB_STATE+$2CA
ls_te               .equ LIB_STATE+$2CD
LS_TITLES           .equ 1
LS_GLYPHS           .equ 2
LS_TREES            .equ 4
LS_FONT             .equ 8
LS_FIRST            .equ 6

ls_lo               .equ lr_n
ls_hi               .equ lr_sr
ls_mid              .equ lr_q
ls_k                .equ lr_cn

LIB_OK              .equ 0
LIB_ERR_RANGE       .equ 1
LIB_ERR_ARG         .equ 2
LIB_ERR_BAD         .equ 3
LIB_ERR_CLOSED      .equ 4
LIB_ERR_INDEX       .equ 5
LIB_ERR_TRAILER     .equ 6
LIB_ERR_TABLE       .equ 7
LIB_ERR_SECTION     .equ 8
LIB_ERR_NONE        .equ 9
LIB_ERR_GLYPHS      .equ 10
