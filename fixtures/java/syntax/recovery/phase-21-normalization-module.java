module recovered.normalization {
    uses z.Service;
    requires transitive + static a.module;
    uses a.Service;
}
