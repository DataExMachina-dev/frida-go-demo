let v = 0;
const f = DebugSymbol.getFunctionByName("main.(*server).ServeHTTP");
Interceptor.attach(f, {
    onEnter(args) { 
        v += 1;
        if ((v % 10000) == 0) { 
            console.log("called", v);
        }
    }
});
