import java.lang.annotation.Native;

@Deprecated
open module example.module {
    requires transitive static java.sql;
    exports example.api to friend.module;
    opens example.internal to friend.module;
    uses example.Service;
    provides example.Service with example.ServiceImpl;
}
