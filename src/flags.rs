use augeas_sys::*;

bitflags! {
    pub struct Flags: aug_flags {
        const None = aug_flags_AUG_NONE;
        const SaveBackup = aug_flags_AUG_SAVE_BACKUP;
        const SafeNewfile = aug_flags_AUG_SAVE_NEWFILE;
        const Typecheck = aug_flags_AUG_TYPE_CHECK;
        const NoStdInclude = aug_flags_AUG_NO_STDINC;
        const SaveNoop = aug_flags_AUG_SAVE_NOOP;
        const NoLoad = aug_flags_AUG_NO_LOAD;
        const NoModuleAutoload = aug_flags_AUG_NO_MODL_AUTOLOAD;
        const EnableSpan = aug_flags_AUG_ENABLE_SPAN;
        const NoErrorClose = aug_flags_AUG_NO_ERR_CLOSE;
        const TraceModuleLoading = aug_flags_AUG_TRACE_MODULE_LOADING;
    }
}
