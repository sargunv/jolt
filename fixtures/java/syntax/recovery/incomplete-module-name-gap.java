module recovered. {
  uses z.Service;
  requires transitive + static a.module;
  uses a.Service;
}
