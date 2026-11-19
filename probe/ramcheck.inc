; The end of a probe image: checks the probe's RAM against the map regions of the parts the image
; includes. The including file defines PROBE_CLEAR(base, size, message), which fails with the
; message when a region of the probe overlaps base and size, and includes this as its last line.
#ifdef UI_NEXT
PROBE_CLEAR(UI_VARS, UI_VARS_SIZE, "the probe's RAM overlaps UI_VARS")
PROBE_CLEAR(UI_FRAME, UI_FRAME_SIZE, "the probe's RAM overlaps UI_FRAME")
#endif
#ifdef FM_NEXT
PROBE_CLEAR(FM_SCRATCH, FM_SCRATCH_SIZE, "the probe's RAM overlaps FM_SCRATCH")
#endif
#ifdef LR_NEXT
PROBE_CLEAR(LIB_STATE, LIB_STATE_SIZE, "the probe's RAM overlaps LIB_STATE")
#endif
#ifdef KD_NEXT
PROBE_CLEAR(KD_SCRATCH, KD_SCRATCH_SIZE, "the probe's RAM overlaps KD_SCRATCH")
#endif
