## build

on mac os, install xquartz, and then
```
    export LIBRARY_PATH=/opt/X11/lib
    cargo build --release
```

## Design
use DSL for rule to filter windows and other stuff

the grammar can be display by 
`cargo run -- --show-grammar`


filtering windows
```
any(attrs.map_state=Viewable, all(geom.x>2, geom.width>100))
```

id wildcard matching
```
all(id=0x10000??, name=*mutter*, attrs.override_redirect=true)
```

actions
```
attrs.map_state=Viewable: filter;
not(attrs.map_state=Viewable): pin;
```

## TODO

- [x] do idle update
- [x] use DSL to specify filter rule (partially)
- [x] highlight diffs across events
- [ ] pin windows (highlight some windows everlasting) 
- [ ] ignore some events
- [x] (de)serialize rules from/into disk 
- [ ] event timestamp?
- [ ] rule databases (based on serialization)
- [x] cut off long name display 
- [ ] change rules on the fly (so I can change the set of monitored windows without restart)
- [ ] xrandr events may affect definition of visible
- [ ] on macOS, client id should be child of queried window
- [x] query_tree is heavy, need to cache window stack like mutter does
- [ ] expand grammar to support window properties
- [ ] expand grammar to support conditional expression
- [ ] add gui support
- [ ] add log level and detail management
- [ ] on-the-fly rule injection/removal (via socket?)


## ideas
- better event tracing, add tracepoint dynamically, listen to any property/attrs change 
- 2.0 wild idea
    systemtap like dynamic tracing, instead of simple filtering. maybe something like:
    ```
    wminspect -f '
        on:configure_notify(window=by_wm_name:deepin) {
            print event.x, event.y
        }

        on:property_notify(window=root, prop=by_name:client_list) {
            let w = get_window(event.window)
            print event.timestamp
        }

        on:create_notify {
            print wm.client_list_stacking
        }

        on:window(id=0x6400001) above window(id=0x4800003) {
            print "window stack changed"
        }
    '

    ```

