var commonjsGlobal = typeof globalThis !== 'undefined' ? globalThis : typeof window !== 'undefined' ? window : typeof global !== 'undefined' ? global : typeof self !== 'undefined' ? self : {};

function getDefaultExportFromCjs (x) {
	return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, 'default') ? x['default'] : x;
}

var dexie = {exports: {}};

/*
 * Dexie.js - a minimalistic wrapper for IndexedDB
 * ===============================================
 *
 * By David Fahlander, david.fahlander@gmail.com
 *
 * Version 4.0.7, Sun May 26 2024
 *
 * https://dexie.org
 *
 * Apache License Version 2.0, January 2004, http://www.apache.org/licenses/
 */

(function (module, exports) {
	(function (global, factory) {
	    module.exports = factory() ;
	})(commonjsGlobal, (function () {
	    /*! *****************************************************************************
	    Copyright (c) Microsoft Corporation.
	    Permission to use, copy, modify, and/or distribute this software for any
	    purpose with or without fee is hereby granted.
	    THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
	    REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
	    AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
	    INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
	    LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
	    OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
	    PERFORMANCE OF THIS SOFTWARE.
	    ***************************************************************************** */
	    var extendStatics = function(d, b) {
	        extendStatics = Object.setPrototypeOf ||
	            ({ __proto__: [] } instanceof Array && function (d, b) { d.__proto__ = b; }) ||
	            function (d, b) { for (var p in b) if (Object.prototype.hasOwnProperty.call(b, p)) d[p] = b[p]; };
	        return extendStatics(d, b);
	    };
	    function __extends(d, b) {
	        if (typeof b !== "function" && b !== null)
	            throw new TypeError("Class extends value " + String(b) + " is not a constructor or null");
	        extendStatics(d, b);
	        function __() { this.constructor = d; }
	        d.prototype = b === null ? Object.create(b) : (__.prototype = b.prototype, new __());
	    }
	    var __assign = function() {
	        __assign = Object.assign || function __assign(t) {
	            for (var s, i = 1, n = arguments.length; i < n; i++) {
	                s = arguments[i];
	                for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p)) t[p] = s[p];
	            }
	            return t;
	        };
	        return __assign.apply(this, arguments);
	    };
	    function __spreadArray(to, from, pack) {
	        if (pack || arguments.length === 2) for (var i = 0, l = from.length, ar; i < l; i++) {
	            if (ar || !(i in from)) {
	                if (!ar) ar = Array.prototype.slice.call(from, 0, i);
	                ar[i] = from[i];
	            }
	        }
	        return to.concat(ar || Array.prototype.slice.call(from));
	    }

	    var _global = typeof globalThis !== 'undefined' ? globalThis :
	        typeof self !== 'undefined' ? self :
	            typeof window !== 'undefined' ? window :
	                commonjsGlobal;

	    var keys = Object.keys;
	    var isArray = Array.isArray;
	    if (typeof Promise !== 'undefined' && !_global.Promise) {
	        _global.Promise = Promise;
	    }
	    function extend(obj, extension) {
	        if (typeof extension !== 'object')
	            return obj;
	        keys(extension).forEach(function (key) {
	            obj[key] = extension[key];
	        });
	        return obj;
	    }
	    var getProto = Object.getPrototypeOf;
	    var _hasOwn = {}.hasOwnProperty;
	    function hasOwn(obj, prop) {
	        return _hasOwn.call(obj, prop);
	    }
	    function props(proto, extension) {
	        if (typeof extension === 'function')
	            extension = extension(getProto(proto));
	        (typeof Reflect === "undefined" ? keys : Reflect.ownKeys)(extension).forEach(function (key) {
	            setProp(proto, key, extension[key]);
	        });
	    }
	    var defineProperty = Object.defineProperty;
	    function setProp(obj, prop, functionOrGetSet, options) {
	        defineProperty(obj, prop, extend(functionOrGetSet && hasOwn(functionOrGetSet, "get") && typeof functionOrGetSet.get === 'function' ?
	            { get: functionOrGetSet.get, set: functionOrGetSet.set, configurable: true } :
	            { value: functionOrGetSet, configurable: true, writable: true }, options));
	    }
	    function derive(Child) {
	        return {
	            from: function (Parent) {
	                Child.prototype = Object.create(Parent.prototype);
	                setProp(Child.prototype, "constructor", Child);
	                return {
	                    extend: props.bind(null, Child.prototype)
	                };
	            }
	        };
	    }
	    var getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
	    function getPropertyDescriptor(obj, prop) {
	        var pd = getOwnPropertyDescriptor(obj, prop);
	        var proto;
	        return pd || (proto = getProto(obj)) && getPropertyDescriptor(proto, prop);
	    }
	    var _slice = [].slice;
	    function slice(args, start, end) {
	        return _slice.call(args, start, end);
	    }
	    function override(origFunc, overridedFactory) {
	        return overridedFactory(origFunc);
	    }
	    function assert(b) {
	        if (!b)
	            throw new Error("Assertion Failed");
	    }
	    function asap$1(fn) {
	        if (_global.setImmediate)
	            setImmediate(fn);
	        else
	            setTimeout(fn, 0);
	    }
	    function arrayToObject(array, extractor) {
	        return array.reduce(function (result, item, i) {
	            var nameAndValue = extractor(item, i);
	            if (nameAndValue)
	                result[nameAndValue[0]] = nameAndValue[1];
	            return result;
	        }, {});
	    }
	    function getByKeyPath(obj, keyPath) {
	        if (typeof keyPath === 'string' && hasOwn(obj, keyPath))
	            return obj[keyPath];
	        if (!keyPath)
	            return obj;
	        if (typeof keyPath !== 'string') {
	            var rv = [];
	            for (var i = 0, l = keyPath.length; i < l; ++i) {
	                var val = getByKeyPath(obj, keyPath[i]);
	                rv.push(val);
	            }
	            return rv;
	        }
	        var period = keyPath.indexOf('.');
	        if (period !== -1) {
	            var innerObj = obj[keyPath.substr(0, period)];
	            return innerObj == null ? undefined : getByKeyPath(innerObj, keyPath.substr(period + 1));
	        }
	        return undefined;
	    }
	    function setByKeyPath(obj, keyPath, value) {
	        if (!obj || keyPath === undefined)
	            return;
	        if ('isFrozen' in Object && Object.isFrozen(obj))
	            return;
	        if (typeof keyPath !== 'string' && 'length' in keyPath) {
	            assert(typeof value !== 'string' && 'length' in value);
	            for (var i = 0, l = keyPath.length; i < l; ++i) {
	                setByKeyPath(obj, keyPath[i], value[i]);
	            }
	        }
	        else {
	            var period = keyPath.indexOf('.');
	            if (period !== -1) {
	                var currentKeyPath = keyPath.substr(0, period);
	                var remainingKeyPath = keyPath.substr(period + 1);
	                if (remainingKeyPath === "")
	                    if (value === undefined) {
	                        if (isArray(obj) && !isNaN(parseInt(currentKeyPath)))
	                            obj.splice(currentKeyPath, 1);
	                        else
	                            delete obj[currentKeyPath];
	                    }
	                    else
	                        obj[currentKeyPath] = value;
	                else {
	                    var innerObj = obj[currentKeyPath];
	                    if (!innerObj || !hasOwn(obj, currentKeyPath))
	                        innerObj = (obj[currentKeyPath] = {});
	                    setByKeyPath(innerObj, remainingKeyPath, value);
	                }
	            }
	            else {
	                if (value === undefined) {
	                    if (isArray(obj) && !isNaN(parseInt(keyPath)))
	                        obj.splice(keyPath, 1);
	                    else
	                        delete obj[keyPath];
	                }
	                else
	                    obj[keyPath] = value;
	            }
	        }
	    }
	    function delByKeyPath(obj, keyPath) {
	        if (typeof keyPath === 'string')
	            setByKeyPath(obj, keyPath, undefined);
	        else if ('length' in keyPath)
	            [].map.call(keyPath, function (kp) {
	                setByKeyPath(obj, kp, undefined);
	            });
	    }
	    function shallowClone(obj) {
	        var rv = {};
	        for (var m in obj) {
	            if (hasOwn(obj, m))
	                rv[m] = obj[m];
	        }
	        return rv;
	    }
	    var concat = [].concat;
	    function flatten(a) {
	        return concat.apply([], a);
	    }
	    var intrinsicTypeNames = "BigUint64Array,BigInt64Array,Array,Boolean,String,Date,RegExp,Blob,File,FileList,FileSystemFileHandle,FileSystemDirectoryHandle,ArrayBuffer,DataView,Uint8ClampedArray,ImageBitmap,ImageData,Map,Set,CryptoKey"
	        .split(',').concat(flatten([8, 16, 32, 64].map(function (num) { return ["Int", "Uint", "Float"].map(function (t) { return t + num + "Array"; }); }))).filter(function (t) { return _global[t]; });
	    var intrinsicTypes = new Set(intrinsicTypeNames.map(function (t) { return _global[t]; }));
	    function cloneSimpleObjectTree(o) {
	        var rv = {};
	        for (var k in o)
	            if (hasOwn(o, k)) {
	                var v = o[k];
	                rv[k] = !v || typeof v !== 'object' || intrinsicTypes.has(v.constructor) ? v : cloneSimpleObjectTree(v);
	            }
	        return rv;
	    }
	    function objectIsEmpty(o) {
	        for (var k in o)
	            if (hasOwn(o, k))
	                return false;
	        return true;
	    }
	    var circularRefs = null;
	    function deepClone(any) {
	        circularRefs = new WeakMap();
	        var rv = innerDeepClone(any);
	        circularRefs = null;
	        return rv;
	    }
	    function innerDeepClone(x) {
	        if (!x || typeof x !== 'object')
	            return x;
	        var rv = circularRefs.get(x);
	        if (rv)
	            return rv;
	        if (isArray(x)) {
	            rv = [];
	            circularRefs.set(x, rv);
	            for (var i = 0, l = x.length; i < l; ++i) {
	                rv.push(innerDeepClone(x[i]));
	            }
	        }
	        else if (intrinsicTypes.has(x.constructor)) {
	            rv = x;
	        }
	        else {
	            var proto = getProto(x);
	            rv = proto === Object.prototype ? {} : Object.create(proto);
	            circularRefs.set(x, rv);
	            for (var prop in x) {
	                if (hasOwn(x, prop)) {
	                    rv[prop] = innerDeepClone(x[prop]);
	                }
	            }
	        }
	        return rv;
	    }
	    var toString = {}.toString;
	    function toStringTag(o) {
	        return toString.call(o).slice(8, -1);
	    }
	    var iteratorSymbol = typeof Symbol !== 'undefined' ?
	        Symbol.iterator :
	        '@@iterator';
	    var getIteratorOf = typeof iteratorSymbol === "symbol" ? function (x) {
	        var i;
	        return x != null && (i = x[iteratorSymbol]) && i.apply(x);
	    } : function () { return null; };
	    function delArrayItem(a, x) {
	        var i = a.indexOf(x);
	        if (i >= 0)
	            a.splice(i, 1);
	        return i >= 0;
	    }
	    var NO_CHAR_ARRAY = {};
	    function getArrayOf(arrayLike) {
	        var i, a, x, it;
	        if (arguments.length === 1) {
	            if (isArray(arrayLike))
	                return arrayLike.slice();
	            if (this === NO_CHAR_ARRAY && typeof arrayLike === 'string')
	                return [arrayLike];
	            if ((it = getIteratorOf(arrayLike))) {
	                a = [];
	                while ((x = it.next()), !x.done)
	                    a.push(x.value);
	                return a;
	            }
	            if (arrayLike == null)
	                return [arrayLike];
	            i = arrayLike.length;
	            if (typeof i === 'number') {
	                a = new Array(i);
	                while (i--)
	                    a[i] = arrayLike[i];
	                return a;
	            }
	            return [arrayLike];
	        }
	        i = arguments.length;
	        a = new Array(i);
	        while (i--)
	            a[i] = arguments[i];
	        return a;
	    }
	    var isAsyncFunction = typeof Symbol !== 'undefined'
	        ? function (fn) { return fn[Symbol.toStringTag] === 'AsyncFunction'; }
	        : function () { return false; };

	    var dexieErrorNames = [
	        'Modify',
	        'Bulk',
	        'OpenFailed',
	        'VersionChange',
	        'Schema',
	        'Upgrade',
	        'InvalidTable',
	        'MissingAPI',
	        'NoSuchDatabase',
	        'InvalidArgument',
	        'SubTransaction',
	        'Unsupported',
	        'Internal',
	        'DatabaseClosed',
	        'PrematureCommit',
	        'ForeignAwait'
	    ];
	    var idbDomErrorNames = [
	        'Unknown',
	        'Constraint',
	        'Data',
	        'TransactionInactive',
	        'ReadOnly',
	        'Version',
	        'NotFound',
	        'InvalidState',
	        'InvalidAccess',
	        'Abort',
	        'Timeout',
	        'QuotaExceeded',
	        'Syntax',
	        'DataClone'
	    ];
	    var errorList = dexieErrorNames.concat(idbDomErrorNames);
	    var defaultTexts = {
	        VersionChanged: "Database version changed by other database connection",
	        DatabaseClosed: "Database has been closed",
	        Abort: "Transaction aborted",
	        TransactionInactive: "Transaction has already completed or failed",
	        MissingAPI: "IndexedDB API missing. Please visit https://tinyurl.com/y2uuvskb"
	    };
	    function DexieError(name, msg) {
	        this.name = name;
	        this.message = msg;
	    }
	    derive(DexieError).from(Error).extend({
	        toString: function () { return this.name + ": " + this.message; }
	    });
	    function getMultiErrorMessage(msg, failures) {
	        return msg + ". Errors: " + Object.keys(failures)
	            .map(function (key) { return failures[key].toString(); })
	            .filter(function (v, i, s) { return s.indexOf(v) === i; })
	            .join('\n');
	    }
	    function ModifyError(msg, failures, successCount, failedKeys) {
	        this.failures = failures;
	        this.failedKeys = failedKeys;
	        this.successCount = successCount;
	        this.message = getMultiErrorMessage(msg, failures);
	    }
	    derive(ModifyError).from(DexieError);
	    function BulkError(msg, failures) {
	        this.name = "BulkError";
	        this.failures = Object.keys(failures).map(function (pos) { return failures[pos]; });
	        this.failuresByPos = failures;
	        this.message = getMultiErrorMessage(msg, this.failures);
	    }
	    derive(BulkError).from(DexieError);
	    var errnames = errorList.reduce(function (obj, name) { return (obj[name] = name + "Error", obj); }, {});
	    var BaseException = DexieError;
	    var exceptions = errorList.reduce(function (obj, name) {
	        var fullName = name + "Error";
	        function DexieError(msgOrInner, inner) {
	            this.name = fullName;
	            if (!msgOrInner) {
	                this.message = defaultTexts[name] || fullName;
	                this.inner = null;
	            }
	            else if (typeof msgOrInner === 'string') {
	                this.message = "".concat(msgOrInner).concat(!inner ? '' : '\n ' + inner);
	                this.inner = inner || null;
	            }
	            else if (typeof msgOrInner === 'object') {
	                this.message = "".concat(msgOrInner.name, " ").concat(msgOrInner.message);
	                this.inner = msgOrInner;
	            }
	        }
	        derive(DexieError).from(BaseException);
	        obj[name] = DexieError;
	        return obj;
	    }, {});
	    exceptions.Syntax = SyntaxError;
	    exceptions.Type = TypeError;
	    exceptions.Range = RangeError;
	    var exceptionMap = idbDomErrorNames.reduce(function (obj, name) {
	        obj[name + "Error"] = exceptions[name];
	        return obj;
	    }, {});
	    function mapError(domError, message) {
	        if (!domError || domError instanceof DexieError || domError instanceof TypeError || domError instanceof SyntaxError || !domError.name || !exceptionMap[domError.name])
	            return domError;
	        var rv = new exceptionMap[domError.name](message || domError.message, domError);
	        if ("stack" in domError) {
	            setProp(rv, "stack", { get: function () {
	                    return this.inner.stack;
	                } });
	        }
	        return rv;
	    }
	    var fullNameExceptions = errorList.reduce(function (obj, name) {
	        if (["Syntax", "Type", "Range"].indexOf(name) === -1)
	            obj[name + "Error"] = exceptions[name];
	        return obj;
	    }, {});
	    fullNameExceptions.ModifyError = ModifyError;
	    fullNameExceptions.DexieError = DexieError;
	    fullNameExceptions.BulkError = BulkError;

	    function nop() { }
	    function mirror(val) { return val; }
	    function pureFunctionChain(f1, f2) {
	        if (f1 == null || f1 === mirror)
	            return f2;
	        return function (val) {
	            return f2(f1(val));
	        };
	    }
	    function callBoth(on1, on2) {
	        return function () {
	            on1.apply(this, arguments);
	            on2.apply(this, arguments);
	        };
	    }
	    function hookCreatingChain(f1, f2) {
	        if (f1 === nop)
	            return f2;
	        return function () {
	            var res = f1.apply(this, arguments);
	            if (res !== undefined)
	                arguments[0] = res;
	            var onsuccess = this.onsuccess,
	            onerror = this.onerror;
	            this.onsuccess = null;
	            this.onerror = null;
	            var res2 = f2.apply(this, arguments);
	            if (onsuccess)
	                this.onsuccess = this.onsuccess ? callBoth(onsuccess, this.onsuccess) : onsuccess;
	            if (onerror)
	                this.onerror = this.onerror ? callBoth(onerror, this.onerror) : onerror;
	            return res2 !== undefined ? res2 : res;
	        };
	    }
	    function hookDeletingChain(f1, f2) {
	        if (f1 === nop)
	            return f2;
	        return function () {
	            f1.apply(this, arguments);
	            var onsuccess = this.onsuccess,
	            onerror = this.onerror;
	            this.onsuccess = this.onerror = null;
	            f2.apply(this, arguments);
	            if (onsuccess)
	                this.onsuccess = this.onsuccess ? callBoth(onsuccess, this.onsuccess) : onsuccess;
	            if (onerror)
	                this.onerror = this.onerror ? callBoth(onerror, this.onerror) : onerror;
	        };
	    }
	    function hookUpdatingChain(f1, f2) {
	        if (f1 === nop)
	            return f2;
	        return function (modifications) {
	            var res = f1.apply(this, arguments);
	            extend(modifications, res);
	            var onsuccess = this.onsuccess,
	            onerror = this.onerror;
	            this.onsuccess = null;
	            this.onerror = null;
	            var res2 = f2.apply(this, arguments);
	            if (onsuccess)
	                this.onsuccess = this.onsuccess ? callBoth(onsuccess, this.onsuccess) : onsuccess;
	            if (onerror)
	                this.onerror = this.onerror ? callBoth(onerror, this.onerror) : onerror;
	            return res === undefined ?
	                (res2 === undefined ? undefined : res2) :
	                (extend(res, res2));
	        };
	    }
	    function reverseStoppableEventChain(f1, f2) {
	        if (f1 === nop)
	            return f2;
	        return function () {
	            if (f2.apply(this, arguments) === false)
	                return false;
	            return f1.apply(this, arguments);
	        };
	    }
	    function promisableChain(f1, f2) {
	        if (f1 === nop)
	            return f2;
	        return function () {
	            var res = f1.apply(this, arguments);
	            if (res && typeof res.then === 'function') {
	                var thiz = this, i = arguments.length, args = new Array(i);
	                while (i--)
	                    args[i] = arguments[i];
	                return res.then(function () {
	                    return f2.apply(thiz, args);
	                });
	            }
	            return f2.apply(this, arguments);
	        };
	    }

	    var debug = typeof location !== 'undefined' &&
	        /^(http|https):\/\/(localhost|127\.0\.0\.1)/.test(location.href);
	    function setDebug(value, filter) {
	        debug = value;
	    }

	    var INTERNAL = {};
	    var ZONE_ECHO_LIMIT = 100, _a$1 = typeof Promise === 'undefined' ?
	        [] :
	        (function () {
	            var globalP = Promise.resolve();
	            if (typeof crypto === 'undefined' || !crypto.subtle)
	                return [globalP, getProto(globalP), globalP];
	            var nativeP = crypto.subtle.digest("SHA-512", new Uint8Array([0]));
	            return [
	                nativeP,
	                getProto(nativeP),
	                globalP
	            ];
	        })(), resolvedNativePromise = _a$1[0], nativePromiseProto = _a$1[1], resolvedGlobalPromise = _a$1[2], nativePromiseThen = nativePromiseProto && nativePromiseProto.then;
	    var NativePromise = resolvedNativePromise && resolvedNativePromise.constructor;
	    var patchGlobalPromise = !!resolvedGlobalPromise;
	    function schedulePhysicalTick() {
	        queueMicrotask(physicalTick);
	    }
	    var asap = function (callback, args) {
	        microtickQueue.push([callback, args]);
	        if (needsNewPhysicalTick) {
	            schedulePhysicalTick();
	            needsNewPhysicalTick = false;
	        }
	    };
	    var isOutsideMicroTick = true,
	    needsNewPhysicalTick = true,
	    unhandledErrors = [],
	    rejectingErrors = [],
	    rejectionMapper = mirror;
	    var globalPSD = {
	        id: 'global',
	        global: true,
	        ref: 0,
	        unhandleds: [],
	        onunhandled: nop,
	        pgp: false,
	        env: {},
	        finalize: nop
	    };
	    var PSD = globalPSD;
	    var microtickQueue = [];
	    var numScheduledCalls = 0;
	    var tickFinalizers = [];
	    function DexiePromise(fn) {
	        if (typeof this !== 'object')
	            throw new TypeError('Promises must be constructed via new');
	        this._listeners = [];
	        this._lib = false;
	        var psd = (this._PSD = PSD);
	        if (typeof fn !== 'function') {
	            if (fn !== INTERNAL)
	                throw new TypeError('Not a function');
	            this._state = arguments[1];
	            this._value = arguments[2];
	            if (this._state === false)
	                handleRejection(this, this._value);
	            return;
	        }
	        this._state = null;
	        this._value = null;
	        ++psd.ref;
	        executePromiseTask(this, fn);
	    }
	    var thenProp = {
	        get: function () {
	            var psd = PSD, microTaskId = totalEchoes;
	            function then(onFulfilled, onRejected) {
	                var _this = this;
	                var possibleAwait = !psd.global && (psd !== PSD || microTaskId !== totalEchoes);
	                var cleanup = possibleAwait && !decrementExpectedAwaits();
	                var rv = new DexiePromise(function (resolve, reject) {
	                    propagateToListener(_this, new Listener(nativeAwaitCompatibleWrap(onFulfilled, psd, possibleAwait, cleanup), nativeAwaitCompatibleWrap(onRejected, psd, possibleAwait, cleanup), resolve, reject, psd));
	                });
	                if (this._consoleTask)
	                    rv._consoleTask = this._consoleTask;
	                return rv;
	            }
	            then.prototype = INTERNAL;
	            return then;
	        },
	        set: function (value) {
	            setProp(this, 'then', value && value.prototype === INTERNAL ?
	                thenProp :
	                {
	                    get: function () {
	                        return value;
	                    },
	                    set: thenProp.set
	                });
	        }
	    };
	    props(DexiePromise.prototype, {
	        then: thenProp,
	        _then: function (onFulfilled, onRejected) {
	            propagateToListener(this, new Listener(null, null, onFulfilled, onRejected, PSD));
	        },
	        catch: function (onRejected) {
	            if (arguments.length === 1)
	                return this.then(null, onRejected);
	            var type = arguments[0], handler = arguments[1];
	            return typeof type === 'function' ? this.then(null, function (err) {
	                return err instanceof type ? handler(err) : PromiseReject(err);
	            })
	                : this.then(null, function (err) {
	                    return err && err.name === type ? handler(err) : PromiseReject(err);
	                });
	        },
	        finally: function (onFinally) {
	            return this.then(function (value) {
	                return DexiePromise.resolve(onFinally()).then(function () { return value; });
	            }, function (err) {
	                return DexiePromise.resolve(onFinally()).then(function () { return PromiseReject(err); });
	            });
	        },
	        timeout: function (ms, msg) {
	            var _this = this;
	            return ms < Infinity ?
	                new DexiePromise(function (resolve, reject) {
	                    var handle = setTimeout(function () { return reject(new exceptions.Timeout(msg)); }, ms);
	                    _this.then(resolve, reject).finally(clearTimeout.bind(null, handle));
	                }) : this;
	        }
	    });
	    if (typeof Symbol !== 'undefined' && Symbol.toStringTag)
	        setProp(DexiePromise.prototype, Symbol.toStringTag, 'Dexie.Promise');
	    globalPSD.env = snapShot();
	    function Listener(onFulfilled, onRejected, resolve, reject, zone) {
	        this.onFulfilled = typeof onFulfilled === 'function' ? onFulfilled : null;
	        this.onRejected = typeof onRejected === 'function' ? onRejected : null;
	        this.resolve = resolve;
	        this.reject = reject;
	        this.psd = zone;
	    }
	    props(DexiePromise, {
	        all: function () {
	            var values = getArrayOf.apply(null, arguments)
	                .map(onPossibleParallellAsync);
	            return new DexiePromise(function (resolve, reject) {
	                if (values.length === 0)
	                    resolve([]);
	                var remaining = values.length;
	                values.forEach(function (a, i) { return DexiePromise.resolve(a).then(function (x) {
	                    values[i] = x;
	                    if (!--remaining)
	                        resolve(values);
	                }, reject); });
	            });
	        },
	        resolve: function (value) {
	            if (value instanceof DexiePromise)
	                return value;
	            if (value && typeof value.then === 'function')
	                return new DexiePromise(function (resolve, reject) {
	                    value.then(resolve, reject);
	                });
	            var rv = new DexiePromise(INTERNAL, true, value);
	            return rv;
	        },
	        reject: PromiseReject,
	        race: function () {
	            var values = getArrayOf.apply(null, arguments).map(onPossibleParallellAsync);
	            return new DexiePromise(function (resolve, reject) {
	                values.map(function (value) { return DexiePromise.resolve(value).then(resolve, reject); });
	            });
	        },
	        PSD: {
	            get: function () { return PSD; },
	            set: function (value) { return PSD = value; }
	        },
	        totalEchoes: { get: function () { return totalEchoes; } },
	        newPSD: newScope,
	        usePSD: usePSD,
	        scheduler: {
	            get: function () { return asap; },
	            set: function (value) { asap = value; }
	        },
	        rejectionMapper: {
	            get: function () { return rejectionMapper; },
	            set: function (value) { rejectionMapper = value; }
	        },
	        follow: function (fn, zoneProps) {
	            return new DexiePromise(function (resolve, reject) {
	                return newScope(function (resolve, reject) {
	                    var psd = PSD;
	                    psd.unhandleds = [];
	                    psd.onunhandled = reject;
	                    psd.finalize = callBoth(function () {
	                        var _this = this;
	                        run_at_end_of_this_or_next_physical_tick(function () {
	                            _this.unhandleds.length === 0 ? resolve() : reject(_this.unhandleds[0]);
	                        });
	                    }, psd.finalize);
	                    fn();
	                }, zoneProps, resolve, reject);
	            });
	        }
	    });
	    if (NativePromise) {
	        if (NativePromise.allSettled)
	            setProp(DexiePromise, "allSettled", function () {
	                var possiblePromises = getArrayOf.apply(null, arguments).map(onPossibleParallellAsync);
	                return new DexiePromise(function (resolve) {
	                    if (possiblePromises.length === 0)
	                        resolve([]);
	                    var remaining = possiblePromises.length;
	                    var results = new Array(remaining);
	                    possiblePromises.forEach(function (p, i) { return DexiePromise.resolve(p).then(function (value) { return results[i] = { status: "fulfilled", value: value }; }, function (reason) { return results[i] = { status: "rejected", reason: reason }; })
	                        .then(function () { return --remaining || resolve(results); }); });
	                });
	            });
	        if (NativePromise.any && typeof AggregateError !== 'undefined')
	            setProp(DexiePromise, "any", function () {
	                var possiblePromises = getArrayOf.apply(null, arguments).map(onPossibleParallellAsync);
	                return new DexiePromise(function (resolve, reject) {
	                    if (possiblePromises.length === 0)
	                        reject(new AggregateError([]));
	                    var remaining = possiblePromises.length;
	                    var failures = new Array(remaining);
	                    possiblePromises.forEach(function (p, i) { return DexiePromise.resolve(p).then(function (value) { return resolve(value); }, function (failure) {
	                        failures[i] = failure;
	                        if (!--remaining)
	                            reject(new AggregateError(failures));
	                    }); });
	                });
	            });
	    }
	    function executePromiseTask(promise, fn) {
	        try {
	            fn(function (value) {
	                if (promise._state !== null)
	                    return;
	                if (value === promise)
	                    throw new TypeError('A promise cannot be resolved with itself.');
	                var shouldExecuteTick = promise._lib && beginMicroTickScope();
	                if (value && typeof value.then === 'function') {
	                    executePromiseTask(promise, function (resolve, reject) {
	                        value instanceof DexiePromise ?
	                            value._then(resolve, reject) :
	                            value.then(resolve, reject);
	                    });
	                }
	                else {
	                    promise._state = true;
	                    promise._value = value;
	                    propagateAllListeners(promise);
	                }
	                if (shouldExecuteTick)
	                    endMicroTickScope();
	            }, handleRejection.bind(null, promise));
	        }
	        catch (ex) {
	            handleRejection(promise, ex);
	        }
	    }
	    function handleRejection(promise, reason) {
	        rejectingErrors.push(reason);
	        if (promise._state !== null)
	            return;
	        var shouldExecuteTick = promise._lib && beginMicroTickScope();
	        reason = rejectionMapper(reason);
	        promise._state = false;
	        promise._value = reason;
	        addPossiblyUnhandledError(promise);
	        propagateAllListeners(promise);
	        if (shouldExecuteTick)
	            endMicroTickScope();
	    }
	    function propagateAllListeners(promise) {
	        var listeners = promise._listeners;
	        promise._listeners = [];
	        for (var i = 0, len = listeners.length; i < len; ++i) {
	            propagateToListener(promise, listeners[i]);
	        }
	        var psd = promise._PSD;
	        --psd.ref || psd.finalize();
	        if (numScheduledCalls === 0) {
	            ++numScheduledCalls;
	            asap(function () {
	                if (--numScheduledCalls === 0)
	                    finalizePhysicalTick();
	            }, []);
	        }
	    }
	    function propagateToListener(promise, listener) {
	        if (promise._state === null) {
	            promise._listeners.push(listener);
	            return;
	        }
	        var cb = promise._state ? listener.onFulfilled : listener.onRejected;
	        if (cb === null) {
	            return (promise._state ? listener.resolve : listener.reject)(promise._value);
	        }
	        ++listener.psd.ref;
	        ++numScheduledCalls;
	        asap(callListener, [cb, promise, listener]);
	    }
	    function callListener(cb, promise, listener) {
	        try {
	            var ret, value = promise._value;
	            if (!promise._state && rejectingErrors.length)
	                rejectingErrors = [];
	            ret = debug && promise._consoleTask ? promise._consoleTask.run(function () { return cb(value); }) : cb(value);
	            if (!promise._state && rejectingErrors.indexOf(value) === -1) {
	                markErrorAsHandled(promise);
	            }
	            listener.resolve(ret);
	        }
	        catch (e) {
	            listener.reject(e);
	        }
	        finally {
	            if (--numScheduledCalls === 0)
	                finalizePhysicalTick();
	            --listener.psd.ref || listener.psd.finalize();
	        }
	    }
	    function physicalTick() {
	        usePSD(globalPSD, function () {
	            beginMicroTickScope() && endMicroTickScope();
	        });
	    }
	    function beginMicroTickScope() {
	        var wasRootExec = isOutsideMicroTick;
	        isOutsideMicroTick = false;
	        needsNewPhysicalTick = false;
	        return wasRootExec;
	    }
	    function endMicroTickScope() {
	        var callbacks, i, l;
	        do {
	            while (microtickQueue.length > 0) {
	                callbacks = microtickQueue;
	                microtickQueue = [];
	                l = callbacks.length;
	                for (i = 0; i < l; ++i) {
	                    var item = callbacks[i];
	                    item[0].apply(null, item[1]);
	                }
	            }
	        } while (microtickQueue.length > 0);
	        isOutsideMicroTick = true;
	        needsNewPhysicalTick = true;
	    }
	    function finalizePhysicalTick() {
	        var unhandledErrs = unhandledErrors;
	        unhandledErrors = [];
	        unhandledErrs.forEach(function (p) {
	            p._PSD.onunhandled.call(null, p._value, p);
	        });
	        var finalizers = tickFinalizers.slice(0);
	        var i = finalizers.length;
	        while (i)
	            finalizers[--i]();
	    }
	    function run_at_end_of_this_or_next_physical_tick(fn) {
	        function finalizer() {
	            fn();
	            tickFinalizers.splice(tickFinalizers.indexOf(finalizer), 1);
	        }
	        tickFinalizers.push(finalizer);
	        ++numScheduledCalls;
	        asap(function () {
	            if (--numScheduledCalls === 0)
	                finalizePhysicalTick();
	        }, []);
	    }
	    function addPossiblyUnhandledError(promise) {
	        if (!unhandledErrors.some(function (p) { return p._value === promise._value; }))
	            unhandledErrors.push(promise);
	    }
	    function markErrorAsHandled(promise) {
	        var i = unhandledErrors.length;
	        while (i)
	            if (unhandledErrors[--i]._value === promise._value) {
	                unhandledErrors.splice(i, 1);
	                return;
	            }
	    }
	    function PromiseReject(reason) {
	        return new DexiePromise(INTERNAL, false, reason);
	    }
	    function wrap(fn, errorCatcher) {
	        var psd = PSD;
	        return function () {
	            var wasRootExec = beginMicroTickScope(), outerScope = PSD;
	            try {
	                switchToZone(psd, true);
	                return fn.apply(this, arguments);
	            }
	            catch (e) {
	                errorCatcher && errorCatcher(e);
	            }
	            finally {
	                switchToZone(outerScope, false);
	                if (wasRootExec)
	                    endMicroTickScope();
	            }
	        };
	    }
	    var task = { awaits: 0, echoes: 0, id: 0 };
	    var taskCounter = 0;
	    var zoneStack = [];
	    var zoneEchoes = 0;
	    var totalEchoes = 0;
	    var zone_id_counter = 0;
	    function newScope(fn, props, a1, a2) {
	        var parent = PSD, psd = Object.create(parent);
	        psd.parent = parent;
	        psd.ref = 0;
	        psd.global = false;
	        psd.id = ++zone_id_counter;
	        globalPSD.env;
	        psd.env = patchGlobalPromise ? {
	            Promise: DexiePromise,
	            PromiseProp: { value: DexiePromise, configurable: true, writable: true },
	            all: DexiePromise.all,
	            race: DexiePromise.race,
	            allSettled: DexiePromise.allSettled,
	            any: DexiePromise.any,
	            resolve: DexiePromise.resolve,
	            reject: DexiePromise.reject,
	        } : {};
	        if (props)
	            extend(psd, props);
	        ++parent.ref;
	        psd.finalize = function () {
	            --this.parent.ref || this.parent.finalize();
	        };
	        var rv = usePSD(psd, fn, a1, a2);
	        if (psd.ref === 0)
	            psd.finalize();
	        return rv;
	    }
	    function incrementExpectedAwaits() {
	        if (!task.id)
	            task.id = ++taskCounter;
	        ++task.awaits;
	        task.echoes += ZONE_ECHO_LIMIT;
	        return task.id;
	    }
	    function decrementExpectedAwaits() {
	        if (!task.awaits)
	            return false;
	        if (--task.awaits === 0)
	            task.id = 0;
	        task.echoes = task.awaits * ZONE_ECHO_LIMIT;
	        return true;
	    }
	    if (('' + nativePromiseThen).indexOf('[native code]') === -1) {
	        incrementExpectedAwaits = decrementExpectedAwaits = nop;
	    }
	    function onPossibleParallellAsync(possiblePromise) {
	        if (task.echoes && possiblePromise && possiblePromise.constructor === NativePromise) {
	            incrementExpectedAwaits();
	            return possiblePromise.then(function (x) {
	                decrementExpectedAwaits();
	                return x;
	            }, function (e) {
	                decrementExpectedAwaits();
	                return rejection(e);
	            });
	        }
	        return possiblePromise;
	    }
	    function zoneEnterEcho(targetZone) {
	        ++totalEchoes;
	        if (!task.echoes || --task.echoes === 0) {
	            task.echoes = task.awaits = task.id = 0;
	        }
	        zoneStack.push(PSD);
	        switchToZone(targetZone, true);
	    }
	    function zoneLeaveEcho() {
	        var zone = zoneStack[zoneStack.length - 1];
	        zoneStack.pop();
	        switchToZone(zone, false);
	    }
	    function switchToZone(targetZone, bEnteringZone) {
	        var currentZone = PSD;
	        if (bEnteringZone ? task.echoes && (!zoneEchoes++ || targetZone !== PSD) : zoneEchoes && (!--zoneEchoes || targetZone !== PSD)) {
	            queueMicrotask(bEnteringZone ? zoneEnterEcho.bind(null, targetZone) : zoneLeaveEcho);
	        }
	        if (targetZone === PSD)
	            return;
	        PSD = targetZone;
	        if (currentZone === globalPSD)
	            globalPSD.env = snapShot();
	        if (patchGlobalPromise) {
	            var GlobalPromise = globalPSD.env.Promise;
	            var targetEnv = targetZone.env;
	            if (currentZone.global || targetZone.global) {
	                Object.defineProperty(_global, 'Promise', targetEnv.PromiseProp);
	                GlobalPromise.all = targetEnv.all;
	                GlobalPromise.race = targetEnv.race;
	                GlobalPromise.resolve = targetEnv.resolve;
	                GlobalPromise.reject = targetEnv.reject;
	                if (targetEnv.allSettled)
	                    GlobalPromise.allSettled = targetEnv.allSettled;
	                if (targetEnv.any)
	                    GlobalPromise.any = targetEnv.any;
	            }
	        }
	    }
	    function snapShot() {
	        var GlobalPromise = _global.Promise;
	        return patchGlobalPromise ? {
	            Promise: GlobalPromise,
	            PromiseProp: Object.getOwnPropertyDescriptor(_global, "Promise"),
	            all: GlobalPromise.all,
	            race: GlobalPromise.race,
	            allSettled: GlobalPromise.allSettled,
	            any: GlobalPromise.any,
	            resolve: GlobalPromise.resolve,
	            reject: GlobalPromise.reject,
	        } : {};
	    }
	    function usePSD(psd, fn, a1, a2, a3) {
	        var outerScope = PSD;
	        try {
	            switchToZone(psd, true);
	            return fn(a1, a2, a3);
	        }
	        finally {
	            switchToZone(outerScope, false);
	        }
	    }
	    function nativeAwaitCompatibleWrap(fn, zone, possibleAwait, cleanup) {
	        return typeof fn !== 'function' ? fn : function () {
	            var outerZone = PSD;
	            if (possibleAwait)
	                incrementExpectedAwaits();
	            switchToZone(zone, true);
	            try {
	                return fn.apply(this, arguments);
	            }
	            finally {
	                switchToZone(outerZone, false);
	                if (cleanup)
	                    queueMicrotask(decrementExpectedAwaits);
	            }
	        };
	    }
	    function execInGlobalContext(cb) {
	        if (Promise === NativePromise && task.echoes === 0) {
	            if (zoneEchoes === 0) {
	                cb();
	            }
	            else {
	                enqueueNativeMicroTask(cb);
	            }
	        }
	        else {
	            setTimeout(cb, 0);
	        }
	    }
	    var rejection = DexiePromise.reject;

	    function tempTransaction(db, mode, storeNames, fn) {
	        if (!db.idbdb || (!db._state.openComplete && (!PSD.letThrough && !db._vip))) {
	            if (db._state.openComplete) {
	                return rejection(new exceptions.DatabaseClosed(db._state.dbOpenError));
	            }
	            if (!db._state.isBeingOpened) {
	                if (!db._state.autoOpen)
	                    return rejection(new exceptions.DatabaseClosed());
	                db.open().catch(nop);
	            }
	            return db._state.dbReadyPromise.then(function () { return tempTransaction(db, mode, storeNames, fn); });
	        }
	        else {
	            var trans = db._createTransaction(mode, storeNames, db._dbSchema);
	            try {
	                trans.create();
	                db._state.PR1398_maxLoop = 3;
	            }
	            catch (ex) {
	                if (ex.name === errnames.InvalidState && db.isOpen() && --db._state.PR1398_maxLoop > 0) {
	                    console.warn('Dexie: Need to reopen db');
	                    db.close({ disableAutoOpen: false });
	                    return db.open().then(function () { return tempTransaction(db, mode, storeNames, fn); });
	                }
	                return rejection(ex);
	            }
	            return trans._promise(mode, function (resolve, reject) {
	                return newScope(function () {
	                    PSD.trans = trans;
	                    return fn(resolve, reject, trans);
	                });
	            }).then(function (result) {
	                if (mode === 'readwrite')
	                    try {
	                        trans.idbtrans.commit();
	                    }
	                    catch (_a) { }
	                return mode === 'readonly' ? result : trans._completion.then(function () { return result; });
	            });
	        }
	    }

	    var DEXIE_VERSION = '4.0.7';
	    var maxString = String.fromCharCode(65535);
	    var minKey = -Infinity;
	    var INVALID_KEY_ARGUMENT = "Invalid key provided. Keys must be of type string, number, Date or Array<string | number | Date>.";
	    var STRING_EXPECTED = "String expected.";
	    var connections = [];
	    var DBNAMES_DB = '__dbnames';
	    var READONLY = 'readonly';
	    var READWRITE = 'readwrite';

	    function combine(filter1, filter2) {
	        return filter1 ?
	            filter2 ?
	                function () { return filter1.apply(this, arguments) && filter2.apply(this, arguments); } :
	                filter1 :
	            filter2;
	    }

	    var AnyRange = {
	        type: 3 ,
	        lower: -Infinity,
	        lowerOpen: false,
	        upper: [[]],
	        upperOpen: false
	    };

	    function workaroundForUndefinedPrimKey(keyPath) {
	        return typeof keyPath === "string" && !/\./.test(keyPath)
	            ? function (obj) {
	                if (obj[keyPath] === undefined && (keyPath in obj)) {
	                    obj = deepClone(obj);
	                    delete obj[keyPath];
	                }
	                return obj;
	            }
	            : function (obj) { return obj; };
	    }

	    function Entity() {
	        throw exceptions.Type();
	    }

	    function cmp(a, b) {
	        try {
	            var ta = type(a);
	            var tb = type(b);
	            if (ta !== tb) {
	                if (ta === 'Array')
	                    return 1;
	                if (tb === 'Array')
	                    return -1;
	                if (ta === 'binary')
	                    return 1;
	                if (tb === 'binary')
	                    return -1;
	                if (ta === 'string')
	                    return 1;
	                if (tb === 'string')
	                    return -1;
	                if (ta === 'Date')
	                    return 1;
	                if (tb !== 'Date')
	                    return NaN;
	                return -1;
	            }
	            switch (ta) {
	                case 'number':
	                case 'Date':
	                case 'string':
	                    return a > b ? 1 : a < b ? -1 : 0;
	                case 'binary': {
	                    return compareUint8Arrays(getUint8Array(a), getUint8Array(b));
	                }
	                case 'Array':
	                    return compareArrays(a, b);
	            }
	        }
	        catch (_a) { }
	        return NaN;
	    }
	    function compareArrays(a, b) {
	        var al = a.length;
	        var bl = b.length;
	        var l = al < bl ? al : bl;
	        for (var i = 0; i < l; ++i) {
	            var res = cmp(a[i], b[i]);
	            if (res !== 0)
	                return res;
	        }
	        return al === bl ? 0 : al < bl ? -1 : 1;
	    }
	    function compareUint8Arrays(a, b) {
	        var al = a.length;
	        var bl = b.length;
	        var l = al < bl ? al : bl;
	        for (var i = 0; i < l; ++i) {
	            if (a[i] !== b[i])
	                return a[i] < b[i] ? -1 : 1;
	        }
	        return al === bl ? 0 : al < bl ? -1 : 1;
	    }
	    function type(x) {
	        var t = typeof x;
	        if (t !== 'object')
	            return t;
	        if (ArrayBuffer.isView(x))
	            return 'binary';
	        var tsTag = toStringTag(x);
	        return tsTag === 'ArrayBuffer' ? 'binary' : tsTag;
	    }
	    function getUint8Array(a) {
	        if (a instanceof Uint8Array)
	            return a;
	        if (ArrayBuffer.isView(a))
	            return new Uint8Array(a.buffer, a.byteOffset, a.byteLength);
	        return new Uint8Array(a);
	    }

	    var Table =  (function () {
	        function Table() {
	        }
	        Table.prototype._trans = function (mode, fn, writeLocked) {
	            var trans = this._tx || PSD.trans;
	            var tableName = this.name;
	            var task = debug && typeof console !== 'undefined' && console.createTask && console.createTask("Dexie: ".concat(mode === 'readonly' ? 'read' : 'write', " ").concat(this.name));
	            function checkTableInTransaction(resolve, reject, trans) {
	                if (!trans.schema[tableName])
	                    throw new exceptions.NotFound("Table " + tableName + " not part of transaction");
	                return fn(trans.idbtrans, trans);
	            }
	            var wasRootExec = beginMicroTickScope();
	            try {
	                var p = trans && trans.db._novip === this.db._novip ?
	                    trans === PSD.trans ?
	                        trans._promise(mode, checkTableInTransaction, writeLocked) :
	                        newScope(function () { return trans._promise(mode, checkTableInTransaction, writeLocked); }, { trans: trans, transless: PSD.transless || PSD }) :
	                    tempTransaction(this.db, mode, [this.name], checkTableInTransaction);
	                if (task) {
	                    p._consoleTask = task;
	                    p = p.catch(function (err) {
	                        console.trace(err);
	                        return rejection(err);
	                    });
	                }
	                return p;
	            }
	            finally {
	                if (wasRootExec)
	                    endMicroTickScope();
	            }
	        };
	        Table.prototype.get = function (keyOrCrit, cb) {
	            var _this = this;
	            if (keyOrCrit && keyOrCrit.constructor === Object)
	                return this.where(keyOrCrit).first(cb);
	            if (keyOrCrit == null)
	                return rejection(new exceptions.Type("Invalid argument to Table.get()"));
	            return this._trans('readonly', function (trans) {
	                return _this.core.get({ trans: trans, key: keyOrCrit })
	                    .then(function (res) { return _this.hook.reading.fire(res); });
	            }).then(cb);
	        };
	        Table.prototype.where = function (indexOrCrit) {
	            if (typeof indexOrCrit === 'string')
	                return new this.db.WhereClause(this, indexOrCrit);
	            if (isArray(indexOrCrit))
	                return new this.db.WhereClause(this, "[".concat(indexOrCrit.join('+'), "]"));
	            var keyPaths = keys(indexOrCrit);
	            if (keyPaths.length === 1)
	                return this
	                    .where(keyPaths[0])
	                    .equals(indexOrCrit[keyPaths[0]]);
	            var compoundIndex = this.schema.indexes.concat(this.schema.primKey).filter(function (ix) {
	                if (ix.compound &&
	                    keyPaths.every(function (keyPath) { return ix.keyPath.indexOf(keyPath) >= 0; })) {
	                    for (var i = 0; i < keyPaths.length; ++i) {
	                        if (keyPaths.indexOf(ix.keyPath[i]) === -1)
	                            return false;
	                    }
	                    return true;
	                }
	                return false;
	            }).sort(function (a, b) { return a.keyPath.length - b.keyPath.length; })[0];
	            if (compoundIndex && this.db._maxKey !== maxString) {
	                var keyPathsInValidOrder = compoundIndex.keyPath.slice(0, keyPaths.length);
	                return this
	                    .where(keyPathsInValidOrder)
	                    .equals(keyPathsInValidOrder.map(function (kp) { return indexOrCrit[kp]; }));
	            }
	            if (!compoundIndex && debug)
	                console.warn("The query ".concat(JSON.stringify(indexOrCrit), " on ").concat(this.name, " would benefit from a ") +
	                    "compound index [".concat(keyPaths.join('+'), "]"));
	            var idxByName = this.schema.idxByName;
	            var idb = this.db._deps.indexedDB;
	            function equals(a, b) {
	                return idb.cmp(a, b) === 0;
	            }
	            var _a = keyPaths.reduce(function (_a, keyPath) {
	                var prevIndex = _a[0], prevFilterFn = _a[1];
	                var index = idxByName[keyPath];
	                var value = indexOrCrit[keyPath];
	                return [
	                    prevIndex || index,
	                    prevIndex || !index ?
	                        combine(prevFilterFn, index && index.multi ?
	                            function (x) {
	                                var prop = getByKeyPath(x, keyPath);
	                                return isArray(prop) && prop.some(function (item) { return equals(value, item); });
	                            } : function (x) { return equals(value, getByKeyPath(x, keyPath)); })
	                        : prevFilterFn
	                ];
	            }, [null, null]), idx = _a[0], filterFunction = _a[1];
	            return idx ?
	                this.where(idx.name).equals(indexOrCrit[idx.keyPath])
	                    .filter(filterFunction) :
	                compoundIndex ?
	                    this.filter(filterFunction) :
	                    this.where(keyPaths).equals('');
	        };
	        Table.prototype.filter = function (filterFunction) {
	            return this.toCollection().and(filterFunction);
	        };
	        Table.prototype.count = function (thenShortcut) {
	            return this.toCollection().count(thenShortcut);
	        };
	        Table.prototype.offset = function (offset) {
	            return this.toCollection().offset(offset);
	        };
	        Table.prototype.limit = function (numRows) {
	            return this.toCollection().limit(numRows);
	        };
	        Table.prototype.each = function (callback) {
	            return this.toCollection().each(callback);
	        };
	        Table.prototype.toArray = function (thenShortcut) {
	            return this.toCollection().toArray(thenShortcut);
	        };
	        Table.prototype.toCollection = function () {
	            return new this.db.Collection(new this.db.WhereClause(this));
	        };
	        Table.prototype.orderBy = function (index) {
	            return new this.db.Collection(new this.db.WhereClause(this, isArray(index) ?
	                "[".concat(index.join('+'), "]") :
	                index));
	        };
	        Table.prototype.reverse = function () {
	            return this.toCollection().reverse();
	        };
	        Table.prototype.mapToClass = function (constructor) {
	            var _a = this, db = _a.db, tableName = _a.name;
	            this.schema.mappedClass = constructor;
	            if (constructor.prototype instanceof Entity) {
	                constructor =  (function (_super) {
	                    __extends(class_1, _super);
	                    function class_1() {
	                        return _super !== null && _super.apply(this, arguments) || this;
	                    }
	                    Object.defineProperty(class_1.prototype, "db", {
	                        get: function () { return db; },
	                        enumerable: false,
	                        configurable: true
	                    });
	                    class_1.prototype.table = function () { return tableName; };
	                    return class_1;
	                }(constructor));
	            }
	            var inheritedProps = new Set();
	            for (var proto = constructor.prototype; proto; proto = getProto(proto)) {
	                Object.getOwnPropertyNames(proto).forEach(function (propName) { return inheritedProps.add(propName); });
	            }
	            var readHook = function (obj) {
	                if (!obj)
	                    return obj;
	                var res = Object.create(constructor.prototype);
	                for (var m in obj)
	                    if (!inheritedProps.has(m))
	                        try {
	                            res[m] = obj[m];
	                        }
	                        catch (_) { }
	                return res;
	            };
	            if (this.schema.readHook) {
	                this.hook.reading.unsubscribe(this.schema.readHook);
	            }
	            this.schema.readHook = readHook;
	            this.hook("reading", readHook);
	            return constructor;
	        };
	        Table.prototype.defineClass = function () {
	            function Class(content) {
	                extend(this, content);
	            }
	            return this.mapToClass(Class);
	        };
	        Table.prototype.add = function (obj, key) {
	            var _this = this;
	            var _a = this.schema.primKey, auto = _a.auto, keyPath = _a.keyPath;
	            var objToAdd = obj;
	            if (keyPath && auto) {
	                objToAdd = workaroundForUndefinedPrimKey(keyPath)(obj);
	            }
	            return this._trans('readwrite', function (trans) {
	                return _this.core.mutate({ trans: trans, type: 'add', keys: key != null ? [key] : null, values: [objToAdd] });
	            }).then(function (res) { return res.numFailures ? DexiePromise.reject(res.failures[0]) : res.lastResult; })
	                .then(function (lastResult) {
	                if (keyPath) {
	                    try {
	                        setByKeyPath(obj, keyPath, lastResult);
	                    }
	                    catch (_) { }
	                }
	                return lastResult;
	            });
	        };
	        Table.prototype.update = function (keyOrObject, modifications) {
	            if (typeof keyOrObject === 'object' && !isArray(keyOrObject)) {
	                var key = getByKeyPath(keyOrObject, this.schema.primKey.keyPath);
	                if (key === undefined)
	                    return rejection(new exceptions.InvalidArgument("Given object does not contain its primary key"));
	                return this.where(":id").equals(key).modify(modifications);
	            }
	            else {
	                return this.where(":id").equals(keyOrObject).modify(modifications);
	            }
	        };
	        Table.prototype.put = function (obj, key) {
	            var _this = this;
	            var _a = this.schema.primKey, auto = _a.auto, keyPath = _a.keyPath;
	            var objToAdd = obj;
	            if (keyPath && auto) {
	                objToAdd = workaroundForUndefinedPrimKey(keyPath)(obj);
	            }
	            return this._trans('readwrite', function (trans) { return _this.core.mutate({ trans: trans, type: 'put', values: [objToAdd], keys: key != null ? [key] : null }); })
	                .then(function (res) { return res.numFailures ? DexiePromise.reject(res.failures[0]) : res.lastResult; })
	                .then(function (lastResult) {
	                if (keyPath) {
	                    try {
	                        setByKeyPath(obj, keyPath, lastResult);
	                    }
	                    catch (_) { }
	                }
	                return lastResult;
	            });
	        };
	        Table.prototype.delete = function (key) {
	            var _this = this;
	            return this._trans('readwrite', function (trans) { return _this.core.mutate({ trans: trans, type: 'delete', keys: [key] }); })
	                .then(function (res) { return res.numFailures ? DexiePromise.reject(res.failures[0]) : undefined; });
	        };
	        Table.prototype.clear = function () {
	            var _this = this;
	            return this._trans('readwrite', function (trans) { return _this.core.mutate({ trans: trans, type: 'deleteRange', range: AnyRange }); })
	                .then(function (res) { return res.numFailures ? DexiePromise.reject(res.failures[0]) : undefined; });
	        };
	        Table.prototype.bulkGet = function (keys) {
	            var _this = this;
	            return this._trans('readonly', function (trans) {
	                return _this.core.getMany({
	                    keys: keys,
	                    trans: trans
	                }).then(function (result) { return result.map(function (res) { return _this.hook.reading.fire(res); }); });
	            });
	        };
	        Table.prototype.bulkAdd = function (objects, keysOrOptions, options) {
	            var _this = this;
	            var keys = Array.isArray(keysOrOptions) ? keysOrOptions : undefined;
	            options = options || (keys ? undefined : keysOrOptions);
	            var wantResults = options ? options.allKeys : undefined;
	            return this._trans('readwrite', function (trans) {
	                var _a = _this.schema.primKey, auto = _a.auto, keyPath = _a.keyPath;
	                if (keyPath && keys)
	                    throw new exceptions.InvalidArgument("bulkAdd(): keys argument invalid on tables with inbound keys");
	                if (keys && keys.length !== objects.length)
	                    throw new exceptions.InvalidArgument("Arguments objects and keys must have the same length");
	                var numObjects = objects.length;
	                var objectsToAdd = keyPath && auto ?
	                    objects.map(workaroundForUndefinedPrimKey(keyPath)) :
	                    objects;
	                return _this.core.mutate({ trans: trans, type: 'add', keys: keys, values: objectsToAdd, wantResults: wantResults })
	                    .then(function (_a) {
	                    var numFailures = _a.numFailures, results = _a.results, lastResult = _a.lastResult, failures = _a.failures;
	                    var result = wantResults ? results : lastResult;
	                    if (numFailures === 0)
	                        return result;
	                    throw new BulkError("".concat(_this.name, ".bulkAdd(): ").concat(numFailures, " of ").concat(numObjects, " operations failed"), failures);
	                });
	            });
	        };
	        Table.prototype.bulkPut = function (objects, keysOrOptions, options) {
	            var _this = this;
	            var keys = Array.isArray(keysOrOptions) ? keysOrOptions : undefined;
	            options = options || (keys ? undefined : keysOrOptions);
	            var wantResults = options ? options.allKeys : undefined;
	            return this._trans('readwrite', function (trans) {
	                var _a = _this.schema.primKey, auto = _a.auto, keyPath = _a.keyPath;
	                if (keyPath && keys)
	                    throw new exceptions.InvalidArgument("bulkPut(): keys argument invalid on tables with inbound keys");
	                if (keys && keys.length !== objects.length)
	                    throw new exceptions.InvalidArgument("Arguments objects and keys must have the same length");
	                var numObjects = objects.length;
	                var objectsToPut = keyPath && auto ?
	                    objects.map(workaroundForUndefinedPrimKey(keyPath)) :
	                    objects;
	                return _this.core.mutate({ trans: trans, type: 'put', keys: keys, values: objectsToPut, wantResults: wantResults })
	                    .then(function (_a) {
	                    var numFailures = _a.numFailures, results = _a.results, lastResult = _a.lastResult, failures = _a.failures;
	                    var result = wantResults ? results : lastResult;
	                    if (numFailures === 0)
	                        return result;
	                    throw new BulkError("".concat(_this.name, ".bulkPut(): ").concat(numFailures, " of ").concat(numObjects, " operations failed"), failures);
	                });
	            });
	        };
	        Table.prototype.bulkUpdate = function (keysAndChanges) {
	            var _this = this;
	            var coreTable = this.core;
	            var keys = keysAndChanges.map(function (entry) { return entry.key; });
	            var changeSpecs = keysAndChanges.map(function (entry) { return entry.changes; });
	            var offsetMap = [];
	            return this._trans('readwrite', function (trans) {
	                return coreTable.getMany({ trans: trans, keys: keys, cache: 'clone' }).then(function (objs) {
	                    var resultKeys = [];
	                    var resultObjs = [];
	                    keysAndChanges.forEach(function (_a, idx) {
	                        var key = _a.key, changes = _a.changes;
	                        var obj = objs[idx];
	                        if (obj) {
	                            for (var _i = 0, _b = Object.keys(changes); _i < _b.length; _i++) {
	                                var keyPath = _b[_i];
	                                var value = changes[keyPath];
	                                if (keyPath === _this.schema.primKey.keyPath) {
	                                    if (cmp(value, key) !== 0) {
	                                        throw new exceptions.Constraint("Cannot update primary key in bulkUpdate()");
	                                    }
	                                }
	                                else {
	                                    setByKeyPath(obj, keyPath, value);
	                                }
	                            }
	                            offsetMap.push(idx);
	                            resultKeys.push(key);
	                            resultObjs.push(obj);
	                        }
	                    });
	                    var numEntries = resultKeys.length;
	                    return coreTable
	                        .mutate({
	                        trans: trans,
	                        type: 'put',
	                        keys: resultKeys,
	                        values: resultObjs,
	                        updates: {
	                            keys: keys,
	                            changeSpecs: changeSpecs
	                        }
	                    })
	                        .then(function (_a) {
	                        var numFailures = _a.numFailures, failures = _a.failures;
	                        if (numFailures === 0)
	                            return numEntries;
	                        for (var _i = 0, _b = Object.keys(failures); _i < _b.length; _i++) {
	                            var offset = _b[_i];
	                            var mappedOffset = offsetMap[Number(offset)];
	                            if (mappedOffset != null) {
	                                var failure = failures[offset];
	                                delete failures[offset];
	                                failures[mappedOffset] = failure;
	                            }
	                        }
	                        throw new BulkError("".concat(_this.name, ".bulkUpdate(): ").concat(numFailures, " of ").concat(numEntries, " operations failed"), failures);
	                    });
	                });
	            });
	        };
	        Table.prototype.bulkDelete = function (keys) {
	            var _this = this;
	            var numKeys = keys.length;
	            return this._trans('readwrite', function (trans) {
	                return _this.core.mutate({ trans: trans, type: 'delete', keys: keys });
	            }).then(function (_a) {
	                var numFailures = _a.numFailures, lastResult = _a.lastResult, failures = _a.failures;
	                if (numFailures === 0)
	                    return lastResult;
	                throw new BulkError("".concat(_this.name, ".bulkDelete(): ").concat(numFailures, " of ").concat(numKeys, " operations failed"), failures);
	            });
	        };
	        return Table;
	    }());

	    function Events(ctx) {
	        var evs = {};
	        var rv = function (eventName, subscriber) {
	            if (subscriber) {
	                var i = arguments.length, args = new Array(i - 1);
	                while (--i)
	                    args[i - 1] = arguments[i];
	                evs[eventName].subscribe.apply(null, args);
	                return ctx;
	            }
	            else if (typeof (eventName) === 'string') {
	                return evs[eventName];
	            }
	        };
	        rv.addEventType = add;
	        for (var i = 1, l = arguments.length; i < l; ++i) {
	            add(arguments[i]);
	        }
	        return rv;
	        function add(eventName, chainFunction, defaultFunction) {
	            if (typeof eventName === 'object')
	                return addConfiguredEvents(eventName);
	            if (!chainFunction)
	                chainFunction = reverseStoppableEventChain;
	            if (!defaultFunction)
	                defaultFunction = nop;
	            var context = {
	                subscribers: [],
	                fire: defaultFunction,
	                subscribe: function (cb) {
	                    if (context.subscribers.indexOf(cb) === -1) {
	                        context.subscribers.push(cb);
	                        context.fire = chainFunction(context.fire, cb);
	                    }
	                },
	                unsubscribe: function (cb) {
	                    context.subscribers = context.subscribers.filter(function (fn) { return fn !== cb; });
	                    context.fire = context.subscribers.reduce(chainFunction, defaultFunction);
	                }
	            };
	            evs[eventName] = rv[eventName] = context;
	            return context;
	        }
	        function addConfiguredEvents(cfg) {
	            keys(cfg).forEach(function (eventName) {
	                var args = cfg[eventName];
	                if (isArray(args)) {
	                    add(eventName, cfg[eventName][0], cfg[eventName][1]);
	                }
	                else if (args === 'asap') {
	                    var context = add(eventName, mirror, function fire() {
	                        var i = arguments.length, args = new Array(i);
	                        while (i--)
	                            args[i] = arguments[i];
	                        context.subscribers.forEach(function (fn) {
	                            asap$1(function fireEvent() {
	                                fn.apply(null, args);
	                            });
	                        });
	                    });
	                }
	                else
	                    throw new exceptions.InvalidArgument("Invalid event config");
	            });
	        }
	    }

	    function makeClassConstructor(prototype, constructor) {
	        derive(constructor).from({ prototype: prototype });
	        return constructor;
	    }

	    function createTableConstructor(db) {
	        return makeClassConstructor(Table.prototype, function Table(name, tableSchema, trans) {
	            this.db = db;
	            this._tx = trans;
	            this.name = name;
	            this.schema = tableSchema;
	            this.hook = db._allTables[name] ? db._allTables[name].hook : Events(null, {
	                "creating": [hookCreatingChain, nop],
	                "reading": [pureFunctionChain, mirror],
	                "updating": [hookUpdatingChain, nop],
	                "deleting": [hookDeletingChain, nop]
	            });
	        });
	    }

	    function isPlainKeyRange(ctx, ignoreLimitFilter) {
	        return !(ctx.filter || ctx.algorithm || ctx.or) &&
	            (ignoreLimitFilter ? ctx.justLimit : !ctx.replayFilter);
	    }
	    function addFilter(ctx, fn) {
	        ctx.filter = combine(ctx.filter, fn);
	    }
	    function addReplayFilter(ctx, factory, isLimitFilter) {
	        var curr = ctx.replayFilter;
	        ctx.replayFilter = curr ? function () { return combine(curr(), factory()); } : factory;
	        ctx.justLimit = isLimitFilter && !curr;
	    }
	    function addMatchFilter(ctx, fn) {
	        ctx.isMatch = combine(ctx.isMatch, fn);
	    }
	    function getIndexOrStore(ctx, coreSchema) {
	        if (ctx.isPrimKey)
	            return coreSchema.primaryKey;
	        var index = coreSchema.getIndexByKeyPath(ctx.index);
	        if (!index)
	            throw new exceptions.Schema("KeyPath " + ctx.index + " on object store " + coreSchema.name + " is not indexed");
	        return index;
	    }
	    function openCursor(ctx, coreTable, trans) {
	        var index = getIndexOrStore(ctx, coreTable.schema);
	        return coreTable.openCursor({
	            trans: trans,
	            values: !ctx.keysOnly,
	            reverse: ctx.dir === 'prev',
	            unique: !!ctx.unique,
	            query: {
	                index: index,
	                range: ctx.range
	            }
	        });
	    }
	    function iter(ctx, fn, coreTrans, coreTable) {
	        var filter = ctx.replayFilter ? combine(ctx.filter, ctx.replayFilter()) : ctx.filter;
	        if (!ctx.or) {
	            return iterate(openCursor(ctx, coreTable, coreTrans), combine(ctx.algorithm, filter), fn, !ctx.keysOnly && ctx.valueMapper);
	        }
	        else {
	            var set_1 = {};
	            var union = function (item, cursor, advance) {
	                if (!filter || filter(cursor, advance, function (result) { return cursor.stop(result); }, function (err) { return cursor.fail(err); })) {
	                    var primaryKey = cursor.primaryKey;
	                    var key = '' + primaryKey;
	                    if (key === '[object ArrayBuffer]')
	                        key = '' + new Uint8Array(primaryKey);
	                    if (!hasOwn(set_1, key)) {
	                        set_1[key] = true;
	                        fn(item, cursor, advance);
	                    }
	                }
	            };
	            return Promise.all([
	                ctx.or._iterate(union, coreTrans),
	                iterate(openCursor(ctx, coreTable, coreTrans), ctx.algorithm, union, !ctx.keysOnly && ctx.valueMapper)
	            ]);
	        }
	    }
	    function iterate(cursorPromise, filter, fn, valueMapper) {
	        var mappedFn = valueMapper ? function (x, c, a) { return fn(valueMapper(x), c, a); } : fn;
	        var wrappedFn = wrap(mappedFn);
	        return cursorPromise.then(function (cursor) {
	            if (cursor) {
	                return cursor.start(function () {
	                    var c = function () { return cursor.continue(); };
	                    if (!filter || filter(cursor, function (advancer) { return c = advancer; }, function (val) { cursor.stop(val); c = nop; }, function (e) { cursor.fail(e); c = nop; }))
	                        wrappedFn(cursor.value, cursor, function (advancer) { return c = advancer; });
	                    c();
	                });
	            }
	        });
	    }

	    var PropModSymbol = Symbol();
	    var PropModification =  (function () {
	        function PropModification(spec) {
	            Object.assign(this, spec);
	        }
	        PropModification.prototype.execute = function (value) {
	            var _a;
	            if (this.add !== undefined) {
	                var term = this.add;
	                if (isArray(term)) {
	                    return __spreadArray(__spreadArray([], (isArray(value) ? value : []), true), term, true).sort();
	                }
	                if (typeof term === 'number')
	                    return (Number(value) || 0) + term;
	                if (typeof term === 'bigint') {
	                    try {
	                        return BigInt(value) + term;
	                    }
	                    catch (_b) {
	                        return BigInt(0) + term;
	                    }
	                }
	                throw new TypeError("Invalid term ".concat(term));
	            }
	            if (this.remove !== undefined) {
	                var subtrahend_1 = this.remove;
	                if (isArray(subtrahend_1)) {
	                    return isArray(value) ? value.filter(function (item) { return !subtrahend_1.includes(item); }).sort() : [];
	                }
	                if (typeof subtrahend_1 === 'number')
	                    return Number(value) - subtrahend_1;
	                if (typeof subtrahend_1 === 'bigint') {
	                    try {
	                        return BigInt(value) - subtrahend_1;
	                    }
	                    catch (_c) {
	                        return BigInt(0) - subtrahend_1;
	                    }
	                }
	                throw new TypeError("Invalid subtrahend ".concat(subtrahend_1));
	            }
	            var prefixToReplace = (_a = this.replacePrefix) === null || _a === void 0 ? void 0 : _a[0];
	            if (prefixToReplace && typeof value === 'string' && value.startsWith(prefixToReplace)) {
	                return this.replacePrefix[1] + value.substring(prefixToReplace.length);
	            }
	            return value;
	        };
	        return PropModification;
	    }());

	    var Collection =  (function () {
	        function Collection() {
	        }
	        Collection.prototype._read = function (fn, cb) {
	            var ctx = this._ctx;
	            return ctx.error ?
	                ctx.table._trans(null, rejection.bind(null, ctx.error)) :
	                ctx.table._trans('readonly', fn).then(cb);
	        };
	        Collection.prototype._write = function (fn) {
	            var ctx = this._ctx;
	            return ctx.error ?
	                ctx.table._trans(null, rejection.bind(null, ctx.error)) :
	                ctx.table._trans('readwrite', fn, "locked");
	        };
	        Collection.prototype._addAlgorithm = function (fn) {
	            var ctx = this._ctx;
	            ctx.algorithm = combine(ctx.algorithm, fn);
	        };
	        Collection.prototype._iterate = function (fn, coreTrans) {
	            return iter(this._ctx, fn, coreTrans, this._ctx.table.core);
	        };
	        Collection.prototype.clone = function (props) {
	            var rv = Object.create(this.constructor.prototype), ctx = Object.create(this._ctx);
	            if (props)
	                extend(ctx, props);
	            rv._ctx = ctx;
	            return rv;
	        };
	        Collection.prototype.raw = function () {
	            this._ctx.valueMapper = null;
	            return this;
	        };
	        Collection.prototype.each = function (fn) {
	            var ctx = this._ctx;
	            return this._read(function (trans) { return iter(ctx, fn, trans, ctx.table.core); });
	        };
	        Collection.prototype.count = function (cb) {
	            var _this = this;
	            return this._read(function (trans) {
	                var ctx = _this._ctx;
	                var coreTable = ctx.table.core;
	                if (isPlainKeyRange(ctx, true)) {
	                    return coreTable.count({
	                        trans: trans,
	                        query: {
	                            index: getIndexOrStore(ctx, coreTable.schema),
	                            range: ctx.range
	                        }
	                    }).then(function (count) { return Math.min(count, ctx.limit); });
	                }
	                else {
	                    var count = 0;
	                    return iter(ctx, function () { ++count; return false; }, trans, coreTable)
	                        .then(function () { return count; });
	                }
	            }).then(cb);
	        };
	        Collection.prototype.sortBy = function (keyPath, cb) {
	            var parts = keyPath.split('.').reverse(), lastPart = parts[0], lastIndex = parts.length - 1;
	            function getval(obj, i) {
	                if (i)
	                    return getval(obj[parts[i]], i - 1);
	                return obj[lastPart];
	            }
	            var order = this._ctx.dir === "next" ? 1 : -1;
	            function sorter(a, b) {
	                var aVal = getval(a, lastIndex), bVal = getval(b, lastIndex);
	                return aVal < bVal ? -order : aVal > bVal ? order : 0;
	            }
	            return this.toArray(function (a) {
	                return a.sort(sorter);
	            }).then(cb);
	        };
	        Collection.prototype.toArray = function (cb) {
	            var _this = this;
	            return this._read(function (trans) {
	                var ctx = _this._ctx;
	                if (ctx.dir === 'next' && isPlainKeyRange(ctx, true) && ctx.limit > 0) {
	                    var valueMapper_1 = ctx.valueMapper;
	                    var index = getIndexOrStore(ctx, ctx.table.core.schema);
	                    return ctx.table.core.query({
	                        trans: trans,
	                        limit: ctx.limit,
	                        values: true,
	                        query: {
	                            index: index,
	                            range: ctx.range
	                        }
	                    }).then(function (_a) {
	                        var result = _a.result;
	                        return valueMapper_1 ? result.map(valueMapper_1) : result;
	                    });
	                }
	                else {
	                    var a_1 = [];
	                    return iter(ctx, function (item) { return a_1.push(item); }, trans, ctx.table.core).then(function () { return a_1; });
	                }
	            }, cb);
	        };
	        Collection.prototype.offset = function (offset) {
	            var ctx = this._ctx;
	            if (offset <= 0)
	                return this;
	            ctx.offset += offset;
	            if (isPlainKeyRange(ctx)) {
	                addReplayFilter(ctx, function () {
	                    var offsetLeft = offset;
	                    return function (cursor, advance) {
	                        if (offsetLeft === 0)
	                            return true;
	                        if (offsetLeft === 1) {
	                            --offsetLeft;
	                            return false;
	                        }
	                        advance(function () {
	                            cursor.advance(offsetLeft);
	                            offsetLeft = 0;
	                        });
	                        return false;
	                    };
	                });
	            }
	            else {
	                addReplayFilter(ctx, function () {
	                    var offsetLeft = offset;
	                    return function () { return (--offsetLeft < 0); };
	                });
	            }
	            return this;
	        };
	        Collection.prototype.limit = function (numRows) {
	            this._ctx.limit = Math.min(this._ctx.limit, numRows);
	            addReplayFilter(this._ctx, function () {
	                var rowsLeft = numRows;
	                return function (cursor, advance, resolve) {
	                    if (--rowsLeft <= 0)
	                        advance(resolve);
	                    return rowsLeft >= 0;
	                };
	            }, true);
	            return this;
	        };
	        Collection.prototype.until = function (filterFunction, bIncludeStopEntry) {
	            addFilter(this._ctx, function (cursor, advance, resolve) {
	                if (filterFunction(cursor.value)) {
	                    advance(resolve);
	                    return bIncludeStopEntry;
	                }
	                else {
	                    return true;
	                }
	            });
	            return this;
	        };
	        Collection.prototype.first = function (cb) {
	            return this.limit(1).toArray(function (a) { return a[0]; }).then(cb);
	        };
	        Collection.prototype.last = function (cb) {
	            return this.reverse().first(cb);
	        };
	        Collection.prototype.filter = function (filterFunction) {
	            addFilter(this._ctx, function (cursor) {
	                return filterFunction(cursor.value);
	            });
	            addMatchFilter(this._ctx, filterFunction);
	            return this;
	        };
	        Collection.prototype.and = function (filter) {
	            return this.filter(filter);
	        };
	        Collection.prototype.or = function (indexName) {
	            return new this.db.WhereClause(this._ctx.table, indexName, this);
	        };
	        Collection.prototype.reverse = function () {
	            this._ctx.dir = (this._ctx.dir === "prev" ? "next" : "prev");
	            if (this._ondirectionchange)
	                this._ondirectionchange(this._ctx.dir);
	            return this;
	        };
	        Collection.prototype.desc = function () {
	            return this.reverse();
	        };
	        Collection.prototype.eachKey = function (cb) {
	            var ctx = this._ctx;
	            ctx.keysOnly = !ctx.isMatch;
	            return this.each(function (val, cursor) { cb(cursor.key, cursor); });
	        };
	        Collection.prototype.eachUniqueKey = function (cb) {
	            this._ctx.unique = "unique";
	            return this.eachKey(cb);
	        };
	        Collection.prototype.eachPrimaryKey = function (cb) {
	            var ctx = this._ctx;
	            ctx.keysOnly = !ctx.isMatch;
	            return this.each(function (val, cursor) { cb(cursor.primaryKey, cursor); });
	        };
	        Collection.prototype.keys = function (cb) {
	            var ctx = this._ctx;
	            ctx.keysOnly = !ctx.isMatch;
	            var a = [];
	            return this.each(function (item, cursor) {
	                a.push(cursor.key);
	            }).then(function () {
	                return a;
	            }).then(cb);
	        };
	        Collection.prototype.primaryKeys = function (cb) {
	            var ctx = this._ctx;
	            if (ctx.dir === 'next' && isPlainKeyRange(ctx, true) && ctx.limit > 0) {
	                return this._read(function (trans) {
	                    var index = getIndexOrStore(ctx, ctx.table.core.schema);
	                    return ctx.table.core.query({
	                        trans: trans,
	                        values: false,
	                        limit: ctx.limit,
	                        query: {
	                            index: index,
	                            range: ctx.range
	                        }
	                    });
	                }).then(function (_a) {
	                    var result = _a.result;
	                    return result;
	                }).then(cb);
	            }
	            ctx.keysOnly = !ctx.isMatch;
	            var a = [];
	            return this.each(function (item, cursor) {
	                a.push(cursor.primaryKey);
	            }).then(function () {
	                return a;
	            }).then(cb);
	        };
	        Collection.prototype.uniqueKeys = function (cb) {
	            this._ctx.unique = "unique";
	            return this.keys(cb);
	        };
	        Collection.prototype.firstKey = function (cb) {
	            return this.limit(1).keys(function (a) { return a[0]; }).then(cb);
	        };
	        Collection.prototype.lastKey = function (cb) {
	            return this.reverse().firstKey(cb);
	        };
	        Collection.prototype.distinct = function () {
	            var ctx = this._ctx, idx = ctx.index && ctx.table.schema.idxByName[ctx.index];
	            if (!idx || !idx.multi)
	                return this;
	            var set = {};
	            addFilter(this._ctx, function (cursor) {
	                var strKey = cursor.primaryKey.toString();
	                var found = hasOwn(set, strKey);
	                set[strKey] = true;
	                return !found;
	            });
	            return this;
	        };
	        Collection.prototype.modify = function (changes) {
	            var _this = this;
	            var ctx = this._ctx;
	            return this._write(function (trans) {
	                var modifyer;
	                if (typeof changes === 'function') {
	                    modifyer = changes;
	                }
	                else {
	                    var keyPaths = keys(changes);
	                    var numKeys = keyPaths.length;
	                    modifyer = function (item) {
	                        var anythingModified = false;
	                        for (var i = 0; i < numKeys; ++i) {
	                            var keyPath = keyPaths[i];
	                            var val = changes[keyPath];
	                            var origVal = getByKeyPath(item, keyPath);
	                            if (val instanceof PropModification) {
	                                setByKeyPath(item, keyPath, val.execute(origVal));
	                                anythingModified = true;
	                            }
	                            else if (origVal !== val) {
	                                setByKeyPath(item, keyPath, val);
	                                anythingModified = true;
	                            }
	                        }
	                        return anythingModified;
	                    };
	                }
	                var coreTable = ctx.table.core;
	                var _a = coreTable.schema.primaryKey, outbound = _a.outbound, extractKey = _a.extractKey;
	                var limit = _this.db._options.modifyChunkSize || 200;
	                var totalFailures = [];
	                var successCount = 0;
	                var failedKeys = [];
	                var applyMutateResult = function (expectedCount, res) {
	                    var failures = res.failures, numFailures = res.numFailures;
	                    successCount += expectedCount - numFailures;
	                    for (var _i = 0, _a = keys(failures); _i < _a.length; _i++) {
	                        var pos = _a[_i];
	                        totalFailures.push(failures[pos]);
	                    }
	                };
	                return _this.clone().primaryKeys().then(function (keys) {
	                    var criteria = isPlainKeyRange(ctx) &&
	                        ctx.limit === Infinity &&
	                        (typeof changes !== 'function' || changes === deleteCallback) && {
	                        index: ctx.index,
	                        range: ctx.range
	                    };
	                    var nextChunk = function (offset) {
	                        var count = Math.min(limit, keys.length - offset);
	                        return coreTable.getMany({
	                            trans: trans,
	                            keys: keys.slice(offset, offset + count),
	                            cache: "immutable"
	                        }).then(function (values) {
	                            var addValues = [];
	                            var putValues = [];
	                            var putKeys = outbound ? [] : null;
	                            var deleteKeys = [];
	                            for (var i = 0; i < count; ++i) {
	                                var origValue = values[i];
	                                var ctx_1 = {
	                                    value: deepClone(origValue),
	                                    primKey: keys[offset + i]
	                                };
	                                if (modifyer.call(ctx_1, ctx_1.value, ctx_1) !== false) {
	                                    if (ctx_1.value == null) {
	                                        deleteKeys.push(keys[offset + i]);
	                                    }
	                                    else if (!outbound && cmp(extractKey(origValue), extractKey(ctx_1.value)) !== 0) {
	                                        deleteKeys.push(keys[offset + i]);
	                                        addValues.push(ctx_1.value);
	                                    }
	                                    else {
	                                        putValues.push(ctx_1.value);
	                                        if (outbound)
	                                            putKeys.push(keys[offset + i]);
	                                    }
	                                }
	                            }
	                            return Promise.resolve(addValues.length > 0 &&
	                                coreTable.mutate({ trans: trans, type: 'add', values: addValues })
	                                    .then(function (res) {
	                                    for (var pos in res.failures) {
	                                        deleteKeys.splice(parseInt(pos), 1);
	                                    }
	                                    applyMutateResult(addValues.length, res);
	                                })).then(function () { return (putValues.length > 0 || (criteria && typeof changes === 'object')) &&
	                                coreTable.mutate({
	                                    trans: trans,
	                                    type: 'put',
	                                    keys: putKeys,
	                                    values: putValues,
	                                    criteria: criteria,
	                                    changeSpec: typeof changes !== 'function'
	                                        && changes,
	                                    isAdditionalChunk: offset > 0
	                                }).then(function (res) { return applyMutateResult(putValues.length, res); }); }).then(function () { return (deleteKeys.length > 0 || (criteria && changes === deleteCallback)) &&
	                                coreTable.mutate({
	                                    trans: trans,
	                                    type: 'delete',
	                                    keys: deleteKeys,
	                                    criteria: criteria,
	                                    isAdditionalChunk: offset > 0
	                                }).then(function (res) { return applyMutateResult(deleteKeys.length, res); }); }).then(function () {
	                                return keys.length > offset + count && nextChunk(offset + limit);
	                            });
	                        });
	                    };
	                    return nextChunk(0).then(function () {
	                        if (totalFailures.length > 0)
	                            throw new ModifyError("Error modifying one or more objects", totalFailures, successCount, failedKeys);
	                        return keys.length;
	                    });
	                });
	            });
	        };
	        Collection.prototype.delete = function () {
	            var ctx = this._ctx, range = ctx.range;
	            if (isPlainKeyRange(ctx) &&
	                (ctx.isPrimKey || range.type === 3 ))
	             {
	                return this._write(function (trans) {
	                    var primaryKey = ctx.table.core.schema.primaryKey;
	                    var coreRange = range;
	                    return ctx.table.core.count({ trans: trans, query: { index: primaryKey, range: coreRange } }).then(function (count) {
	                        return ctx.table.core.mutate({ trans: trans, type: 'deleteRange', range: coreRange })
	                            .then(function (_a) {
	                            var failures = _a.failures; _a.lastResult; _a.results; var numFailures = _a.numFailures;
	                            if (numFailures)
	                                throw new ModifyError("Could not delete some values", Object.keys(failures).map(function (pos) { return failures[pos]; }), count - numFailures);
	                            return count - numFailures;
	                        });
	                    });
	                });
	            }
	            return this.modify(deleteCallback);
	        };
	        return Collection;
	    }());
	    var deleteCallback = function (value, ctx) { return ctx.value = null; };

	    function createCollectionConstructor(db) {
	        return makeClassConstructor(Collection.prototype, function Collection(whereClause, keyRangeGenerator) {
	            this.db = db;
	            var keyRange = AnyRange, error = null;
	            if (keyRangeGenerator)
	                try {
	                    keyRange = keyRangeGenerator();
	                }
	                catch (ex) {
	                    error = ex;
	                }
	            var whereCtx = whereClause._ctx;
	            var table = whereCtx.table;
	            var readingHook = table.hook.reading.fire;
	            this._ctx = {
	                table: table,
	                index: whereCtx.index,
	                isPrimKey: (!whereCtx.index || (table.schema.primKey.keyPath && whereCtx.index === table.schema.primKey.name)),
	                range: keyRange,
	                keysOnly: false,
	                dir: "next",
	                unique: "",
	                algorithm: null,
	                filter: null,
	                replayFilter: null,
	                justLimit: true,
	                isMatch: null,
	                offset: 0,
	                limit: Infinity,
	                error: error,
	                or: whereCtx.or,
	                valueMapper: readingHook !== mirror ? readingHook : null
	            };
	        });
	    }

	    function simpleCompare(a, b) {
	        return a < b ? -1 : a === b ? 0 : 1;
	    }
	    function simpleCompareReverse(a, b) {
	        return a > b ? -1 : a === b ? 0 : 1;
	    }

	    function fail(collectionOrWhereClause, err, T) {
	        var collection = collectionOrWhereClause instanceof WhereClause ?
	            new collectionOrWhereClause.Collection(collectionOrWhereClause) :
	            collectionOrWhereClause;
	        collection._ctx.error = T ? new T(err) : new TypeError(err);
	        return collection;
	    }
	    function emptyCollection(whereClause) {
	        return new whereClause.Collection(whereClause, function () { return rangeEqual(""); }).limit(0);
	    }
	    function upperFactory(dir) {
	        return dir === "next" ?
	            function (s) { return s.toUpperCase(); } :
	            function (s) { return s.toLowerCase(); };
	    }
	    function lowerFactory(dir) {
	        return dir === "next" ?
	            function (s) { return s.toLowerCase(); } :
	            function (s) { return s.toUpperCase(); };
	    }
	    function nextCasing(key, lowerKey, upperNeedle, lowerNeedle, cmp, dir) {
	        var length = Math.min(key.length, lowerNeedle.length);
	        var llp = -1;
	        for (var i = 0; i < length; ++i) {
	            var lwrKeyChar = lowerKey[i];
	            if (lwrKeyChar !== lowerNeedle[i]) {
	                if (cmp(key[i], upperNeedle[i]) < 0)
	                    return key.substr(0, i) + upperNeedle[i] + upperNeedle.substr(i + 1);
	                if (cmp(key[i], lowerNeedle[i]) < 0)
	                    return key.substr(0, i) + lowerNeedle[i] + upperNeedle.substr(i + 1);
	                if (llp >= 0)
	                    return key.substr(0, llp) + lowerKey[llp] + upperNeedle.substr(llp + 1);
	                return null;
	            }
	            if (cmp(key[i], lwrKeyChar) < 0)
	                llp = i;
	        }
	        if (length < lowerNeedle.length && dir === "next")
	            return key + upperNeedle.substr(key.length);
	        if (length < key.length && dir === "prev")
	            return key.substr(0, upperNeedle.length);
	        return (llp < 0 ? null : key.substr(0, llp) + lowerNeedle[llp] + upperNeedle.substr(llp + 1));
	    }
	    function addIgnoreCaseAlgorithm(whereClause, match, needles, suffix) {
	        var upper, lower, compare, upperNeedles, lowerNeedles, direction, nextKeySuffix, needlesLen = needles.length;
	        if (!needles.every(function (s) { return typeof s === 'string'; })) {
	            return fail(whereClause, STRING_EXPECTED);
	        }
	        function initDirection(dir) {
	            upper = upperFactory(dir);
	            lower = lowerFactory(dir);
	            compare = (dir === "next" ? simpleCompare : simpleCompareReverse);
	            var needleBounds = needles.map(function (needle) {
	                return { lower: lower(needle), upper: upper(needle) };
	            }).sort(function (a, b) {
	                return compare(a.lower, b.lower);
	            });
	            upperNeedles = needleBounds.map(function (nb) { return nb.upper; });
	            lowerNeedles = needleBounds.map(function (nb) { return nb.lower; });
	            direction = dir;
	            nextKeySuffix = (dir === "next" ? "" : suffix);
	        }
	        initDirection("next");
	        var c = new whereClause.Collection(whereClause, function () { return createRange(upperNeedles[0], lowerNeedles[needlesLen - 1] + suffix); });
	        c._ondirectionchange = function (direction) {
	            initDirection(direction);
	        };
	        var firstPossibleNeedle = 0;
	        c._addAlgorithm(function (cursor, advance, resolve) {
	            var key = cursor.key;
	            if (typeof key !== 'string')
	                return false;
	            var lowerKey = lower(key);
	            if (match(lowerKey, lowerNeedles, firstPossibleNeedle)) {
	                return true;
	            }
	            else {
	                var lowestPossibleCasing = null;
	                for (var i = firstPossibleNeedle; i < needlesLen; ++i) {
	                    var casing = nextCasing(key, lowerKey, upperNeedles[i], lowerNeedles[i], compare, direction);
	                    if (casing === null && lowestPossibleCasing === null)
	                        firstPossibleNeedle = i + 1;
	                    else if (lowestPossibleCasing === null || compare(lowestPossibleCasing, casing) > 0) {
	                        lowestPossibleCasing = casing;
	                    }
	                }
	                if (lowestPossibleCasing !== null) {
	                    advance(function () { cursor.continue(lowestPossibleCasing + nextKeySuffix); });
	                }
	                else {
	                    advance(resolve);
	                }
	                return false;
	            }
	        });
	        return c;
	    }
	    function createRange(lower, upper, lowerOpen, upperOpen) {
	        return {
	            type: 2 ,
	            lower: lower,
	            upper: upper,
	            lowerOpen: lowerOpen,
	            upperOpen: upperOpen
	        };
	    }
	    function rangeEqual(value) {
	        return {
	            type: 1 ,
	            lower: value,
	            upper: value
	        };
	    }

	    var WhereClause =  (function () {
	        function WhereClause() {
	        }
	        Object.defineProperty(WhereClause.prototype, "Collection", {
	            get: function () {
	                return this._ctx.table.db.Collection;
	            },
	            enumerable: false,
	            configurable: true
	        });
	        WhereClause.prototype.between = function (lower, upper, includeLower, includeUpper) {
	            includeLower = includeLower !== false;
	            includeUpper = includeUpper === true;
	            try {
	                if ((this._cmp(lower, upper) > 0) ||
	                    (this._cmp(lower, upper) === 0 && (includeLower || includeUpper) && !(includeLower && includeUpper)))
	                    return emptyCollection(this);
	                return new this.Collection(this, function () { return createRange(lower, upper, !includeLower, !includeUpper); });
	            }
	            catch (e) {
	                return fail(this, INVALID_KEY_ARGUMENT);
	            }
	        };
	        WhereClause.prototype.equals = function (value) {
	            if (value == null)
	                return fail(this, INVALID_KEY_ARGUMENT);
	            return new this.Collection(this, function () { return rangeEqual(value); });
	        };
	        WhereClause.prototype.above = function (value) {
	            if (value == null)
	                return fail(this, INVALID_KEY_ARGUMENT);
	            return new this.Collection(this, function () { return createRange(value, undefined, true); });
	        };
	        WhereClause.prototype.aboveOrEqual = function (value) {
	            if (value == null)
	                return fail(this, INVALID_KEY_ARGUMENT);
	            return new this.Collection(this, function () { return createRange(value, undefined, false); });
	        };
	        WhereClause.prototype.below = function (value) {
	            if (value == null)
	                return fail(this, INVALID_KEY_ARGUMENT);
	            return new this.Collection(this, function () { return createRange(undefined, value, false, true); });
	        };
	        WhereClause.prototype.belowOrEqual = function (value) {
	            if (value == null)
	                return fail(this, INVALID_KEY_ARGUMENT);
	            return new this.Collection(this, function () { return createRange(undefined, value); });
	        };
	        WhereClause.prototype.startsWith = function (str) {
	            if (typeof str !== 'string')
	                return fail(this, STRING_EXPECTED);
	            return this.between(str, str + maxString, true, true);
	        };
	        WhereClause.prototype.startsWithIgnoreCase = function (str) {
	            if (str === "")
	                return this.startsWith(str);
	            return addIgnoreCaseAlgorithm(this, function (x, a) { return x.indexOf(a[0]) === 0; }, [str], maxString);
	        };
	        WhereClause.prototype.equalsIgnoreCase = function (str) {
	            return addIgnoreCaseAlgorithm(this, function (x, a) { return x === a[0]; }, [str], "");
	        };
	        WhereClause.prototype.anyOfIgnoreCase = function () {
	            var set = getArrayOf.apply(NO_CHAR_ARRAY, arguments);
	            if (set.length === 0)
	                return emptyCollection(this);
	            return addIgnoreCaseAlgorithm(this, function (x, a) { return a.indexOf(x) !== -1; }, set, "");
	        };
	        WhereClause.prototype.startsWithAnyOfIgnoreCase = function () {
	            var set = getArrayOf.apply(NO_CHAR_ARRAY, arguments);
	            if (set.length === 0)
	                return emptyCollection(this);
	            return addIgnoreCaseAlgorithm(this, function (x, a) { return a.some(function (n) { return x.indexOf(n) === 0; }); }, set, maxString);
	        };
	        WhereClause.prototype.anyOf = function () {
	            var _this = this;
	            var set = getArrayOf.apply(NO_CHAR_ARRAY, arguments);
	            var compare = this._cmp;
	            try {
	                set.sort(compare);
	            }
	            catch (e) {
	                return fail(this, INVALID_KEY_ARGUMENT);
	            }
	            if (set.length === 0)
	                return emptyCollection(this);
	            var c = new this.Collection(this, function () { return createRange(set[0], set[set.length - 1]); });
	            c._ondirectionchange = function (direction) {
	                compare = (direction === "next" ?
	                    _this._ascending :
	                    _this._descending);
	                set.sort(compare);
	            };
	            var i = 0;
	            c._addAlgorithm(function (cursor, advance, resolve) {
	                var key = cursor.key;
	                while (compare(key, set[i]) > 0) {
	                    ++i;
	                    if (i === set.length) {
	                        advance(resolve);
	                        return false;
	                    }
	                }
	                if (compare(key, set[i]) === 0) {
	                    return true;
	                }
	                else {
	                    advance(function () { cursor.continue(set[i]); });
	                    return false;
	                }
	            });
	            return c;
	        };
	        WhereClause.prototype.notEqual = function (value) {
	            return this.inAnyRange([[minKey, value], [value, this.db._maxKey]], { includeLowers: false, includeUppers: false });
	        };
	        WhereClause.prototype.noneOf = function () {
	            var set = getArrayOf.apply(NO_CHAR_ARRAY, arguments);
	            if (set.length === 0)
	                return new this.Collection(this);
	            try {
	                set.sort(this._ascending);
	            }
	            catch (e) {
	                return fail(this, INVALID_KEY_ARGUMENT);
	            }
	            var ranges = set.reduce(function (res, val) { return res ?
	                res.concat([[res[res.length - 1][1], val]]) :
	                [[minKey, val]]; }, null);
	            ranges.push([set[set.length - 1], this.db._maxKey]);
	            return this.inAnyRange(ranges, { includeLowers: false, includeUppers: false });
	        };
	        WhereClause.prototype.inAnyRange = function (ranges, options) {
	            var _this = this;
	            var cmp = this._cmp, ascending = this._ascending, descending = this._descending, min = this._min, max = this._max;
	            if (ranges.length === 0)
	                return emptyCollection(this);
	            if (!ranges.every(function (range) {
	                return range[0] !== undefined &&
	                    range[1] !== undefined &&
	                    ascending(range[0], range[1]) <= 0;
	            })) {
	                return fail(this, "First argument to inAnyRange() must be an Array of two-value Arrays [lower,upper] where upper must not be lower than lower", exceptions.InvalidArgument);
	            }
	            var includeLowers = !options || options.includeLowers !== false;
	            var includeUppers = options && options.includeUppers === true;
	            function addRange(ranges, newRange) {
	                var i = 0, l = ranges.length;
	                for (; i < l; ++i) {
	                    var range = ranges[i];
	                    if (cmp(newRange[0], range[1]) < 0 && cmp(newRange[1], range[0]) > 0) {
	                        range[0] = min(range[0], newRange[0]);
	                        range[1] = max(range[1], newRange[1]);
	                        break;
	                    }
	                }
	                if (i === l)
	                    ranges.push(newRange);
	                return ranges;
	            }
	            var sortDirection = ascending;
	            function rangeSorter(a, b) { return sortDirection(a[0], b[0]); }
	            var set;
	            try {
	                set = ranges.reduce(addRange, []);
	                set.sort(rangeSorter);
	            }
	            catch (ex) {
	                return fail(this, INVALID_KEY_ARGUMENT);
	            }
	            var rangePos = 0;
	            var keyIsBeyondCurrentEntry = includeUppers ?
	                function (key) { return ascending(key, set[rangePos][1]) > 0; } :
	                function (key) { return ascending(key, set[rangePos][1]) >= 0; };
	            var keyIsBeforeCurrentEntry = includeLowers ?
	                function (key) { return descending(key, set[rangePos][0]) > 0; } :
	                function (key) { return descending(key, set[rangePos][0]) >= 0; };
	            function keyWithinCurrentRange(key) {
	                return !keyIsBeyondCurrentEntry(key) && !keyIsBeforeCurrentEntry(key);
	            }
	            var checkKey = keyIsBeyondCurrentEntry;
	            var c = new this.Collection(this, function () { return createRange(set[0][0], set[set.length - 1][1], !includeLowers, !includeUppers); });
	            c._ondirectionchange = function (direction) {
	                if (direction === "next") {
	                    checkKey = keyIsBeyondCurrentEntry;
	                    sortDirection = ascending;
	                }
	                else {
	                    checkKey = keyIsBeforeCurrentEntry;
	                    sortDirection = descending;
	                }
	                set.sort(rangeSorter);
	            };
	            c._addAlgorithm(function (cursor, advance, resolve) {
	                var key = cursor.key;
	                while (checkKey(key)) {
	                    ++rangePos;
	                    if (rangePos === set.length) {
	                        advance(resolve);
	                        return false;
	                    }
	                }
	                if (keyWithinCurrentRange(key)) {
	                    return true;
	                }
	                else if (_this._cmp(key, set[rangePos][1]) === 0 || _this._cmp(key, set[rangePos][0]) === 0) {
	                    return false;
	                }
	                else {
	                    advance(function () {
	                        if (sortDirection === ascending)
	                            cursor.continue(set[rangePos][0]);
	                        else
	                            cursor.continue(set[rangePos][1]);
	                    });
	                    return false;
	                }
	            });
	            return c;
	        };
	        WhereClause.prototype.startsWithAnyOf = function () {
	            var set = getArrayOf.apply(NO_CHAR_ARRAY, arguments);
	            if (!set.every(function (s) { return typeof s === 'string'; })) {
	                return fail(this, "startsWithAnyOf() only works with strings");
	            }
	            if (set.length === 0)
	                return emptyCollection(this);
	            return this.inAnyRange(set.map(function (str) { return [str, str + maxString]; }));
	        };
	        return WhereClause;
	    }());

	    function createWhereClauseConstructor(db) {
	        return makeClassConstructor(WhereClause.prototype, function WhereClause(table, index, orCollection) {
	            this.db = db;
	            this._ctx = {
	                table: table,
	                index: index === ":id" ? null : index,
	                or: orCollection
	            };
	            this._cmp = this._ascending = cmp;
	            this._descending = function (a, b) { return cmp(b, a); };
	            this._max = function (a, b) { return cmp(a, b) > 0 ? a : b; };
	            this._min = function (a, b) { return cmp(a, b) < 0 ? a : b; };
	            this._IDBKeyRange = db._deps.IDBKeyRange;
	            if (!this._IDBKeyRange)
	                throw new exceptions.MissingAPI();
	        });
	    }

	    function eventRejectHandler(reject) {
	        return wrap(function (event) {
	            preventDefault(event);
	            reject(event.target.error);
	            return false;
	        });
	    }
	    function preventDefault(event) {
	        if (event.stopPropagation)
	            event.stopPropagation();
	        if (event.preventDefault)
	            event.preventDefault();
	    }

	    var DEXIE_STORAGE_MUTATED_EVENT_NAME = 'storagemutated';
	    var STORAGE_MUTATED_DOM_EVENT_NAME = 'x-storagemutated-1';
	    var globalEvents = Events(null, DEXIE_STORAGE_MUTATED_EVENT_NAME);

	    var Transaction =  (function () {
	        function Transaction() {
	        }
	        Transaction.prototype._lock = function () {
	            assert(!PSD.global);
	            ++this._reculock;
	            if (this._reculock === 1 && !PSD.global)
	                PSD.lockOwnerFor = this;
	            return this;
	        };
	        Transaction.prototype._unlock = function () {
	            assert(!PSD.global);
	            if (--this._reculock === 0) {
	                if (!PSD.global)
	                    PSD.lockOwnerFor = null;
	                while (this._blockedFuncs.length > 0 && !this._locked()) {
	                    var fnAndPSD = this._blockedFuncs.shift();
	                    try {
	                        usePSD(fnAndPSD[1], fnAndPSD[0]);
	                    }
	                    catch (e) { }
	                }
	            }
	            return this;
	        };
	        Transaction.prototype._locked = function () {
	            return this._reculock && PSD.lockOwnerFor !== this;
	        };
	        Transaction.prototype.create = function (idbtrans) {
	            var _this = this;
	            if (!this.mode)
	                return this;
	            var idbdb = this.db.idbdb;
	            var dbOpenError = this.db._state.dbOpenError;
	            assert(!this.idbtrans);
	            if (!idbtrans && !idbdb) {
	                switch (dbOpenError && dbOpenError.name) {
	                    case "DatabaseClosedError":
	                        throw new exceptions.DatabaseClosed(dbOpenError);
	                    case "MissingAPIError":
	                        throw new exceptions.MissingAPI(dbOpenError.message, dbOpenError);
	                    default:
	                        throw new exceptions.OpenFailed(dbOpenError);
	                }
	            }
	            if (!this.active)
	                throw new exceptions.TransactionInactive();
	            assert(this._completion._state === null);
	            idbtrans = this.idbtrans = idbtrans ||
	                (this.db.core
	                    ? this.db.core.transaction(this.storeNames, this.mode, { durability: this.chromeTransactionDurability })
	                    : idbdb.transaction(this.storeNames, this.mode, { durability: this.chromeTransactionDurability }));
	            idbtrans.onerror = wrap(function (ev) {
	                preventDefault(ev);
	                _this._reject(idbtrans.error);
	            });
	            idbtrans.onabort = wrap(function (ev) {
	                preventDefault(ev);
	                _this.active && _this._reject(new exceptions.Abort(idbtrans.error));
	                _this.active = false;
	                _this.on("abort").fire(ev);
	            });
	            idbtrans.oncomplete = wrap(function () {
	                _this.active = false;
	                _this._resolve();
	                if ('mutatedParts' in idbtrans) {
	                    globalEvents.storagemutated.fire(idbtrans["mutatedParts"]);
	                }
	            });
	            return this;
	        };
	        Transaction.prototype._promise = function (mode, fn, bWriteLock) {
	            var _this = this;
	            if (mode === 'readwrite' && this.mode !== 'readwrite')
	                return rejection(new exceptions.ReadOnly("Transaction is readonly"));
	            if (!this.active)
	                return rejection(new exceptions.TransactionInactive());
	            if (this._locked()) {
	                return new DexiePromise(function (resolve, reject) {
	                    _this._blockedFuncs.push([function () {
	                            _this._promise(mode, fn, bWriteLock).then(resolve, reject);
	                        }, PSD]);
	                });
	            }
	            else if (bWriteLock) {
	                return newScope(function () {
	                    var p = new DexiePromise(function (resolve, reject) {
	                        _this._lock();
	                        var rv = fn(resolve, reject, _this);
	                        if (rv && rv.then)
	                            rv.then(resolve, reject);
	                    });
	                    p.finally(function () { return _this._unlock(); });
	                    p._lib = true;
	                    return p;
	                });
	            }
	            else {
	                var p = new DexiePromise(function (resolve, reject) {
	                    var rv = fn(resolve, reject, _this);
	                    if (rv && rv.then)
	                        rv.then(resolve, reject);
	                });
	                p._lib = true;
	                return p;
	            }
	        };
	        Transaction.prototype._root = function () {
	            return this.parent ? this.parent._root() : this;
	        };
	        Transaction.prototype.waitFor = function (promiseLike) {
	            var root = this._root();
	            var promise = DexiePromise.resolve(promiseLike);
	            if (root._waitingFor) {
	                root._waitingFor = root._waitingFor.then(function () { return promise; });
	            }
	            else {
	                root._waitingFor = promise;
	                root._waitingQueue = [];
	                var store = root.idbtrans.objectStore(root.storeNames[0]);
	                (function spin() {
	                    ++root._spinCount;
	                    while (root._waitingQueue.length)
	                        (root._waitingQueue.shift())();
	                    if (root._waitingFor)
	                        store.get(-Infinity).onsuccess = spin;
	                }());
	            }
	            var currentWaitPromise = root._waitingFor;
	            return new DexiePromise(function (resolve, reject) {
	                promise.then(function (res) { return root._waitingQueue.push(wrap(resolve.bind(null, res))); }, function (err) { return root._waitingQueue.push(wrap(reject.bind(null, err))); }).finally(function () {
	                    if (root._waitingFor === currentWaitPromise) {
	                        root._waitingFor = null;
	                    }
	                });
	            });
	        };
	        Transaction.prototype.abort = function () {
	            if (this.active) {
	                this.active = false;
	                if (this.idbtrans)
	                    this.idbtrans.abort();
	                this._reject(new exceptions.Abort());
	            }
	        };
	        Transaction.prototype.table = function (tableName) {
	            var memoizedTables = (this._memoizedTables || (this._memoizedTables = {}));
	            if (hasOwn(memoizedTables, tableName))
	                return memoizedTables[tableName];
	            var tableSchema = this.schema[tableName];
	            if (!tableSchema) {
	                throw new exceptions.NotFound("Table " + tableName + " not part of transaction");
	            }
	            var transactionBoundTable = new this.db.Table(tableName, tableSchema, this);
	            transactionBoundTable.core = this.db.core.table(tableName);
	            memoizedTables[tableName] = transactionBoundTable;
	            return transactionBoundTable;
	        };
	        return Transaction;
	    }());

	    function createTransactionConstructor(db) {
	        return makeClassConstructor(Transaction.prototype, function Transaction(mode, storeNames, dbschema, chromeTransactionDurability, parent) {
	            var _this = this;
	            this.db = db;
	            this.mode = mode;
	            this.storeNames = storeNames;
	            this.schema = dbschema;
	            this.chromeTransactionDurability = chromeTransactionDurability;
	            this.idbtrans = null;
	            this.on = Events(this, "complete", "error", "abort");
	            this.parent = parent || null;
	            this.active = true;
	            this._reculock = 0;
	            this._blockedFuncs = [];
	            this._resolve = null;
	            this._reject = null;
	            this._waitingFor = null;
	            this._waitingQueue = null;
	            this._spinCount = 0;
	            this._completion = new DexiePromise(function (resolve, reject) {
	                _this._resolve = resolve;
	                _this._reject = reject;
	            });
	            this._completion.then(function () {
	                _this.active = false;
	                _this.on.complete.fire();
	            }, function (e) {
	                var wasActive = _this.active;
	                _this.active = false;
	                _this.on.error.fire(e);
	                _this.parent ?
	                    _this.parent._reject(e) :
	                    wasActive && _this.idbtrans && _this.idbtrans.abort();
	                return rejection(e);
	            });
	        });
	    }

	    function createIndexSpec(name, keyPath, unique, multi, auto, compound, isPrimKey) {
	        return {
	            name: name,
	            keyPath: keyPath,
	            unique: unique,
	            multi: multi,
	            auto: auto,
	            compound: compound,
	            src: (unique && !isPrimKey ? '&' : '') + (multi ? '*' : '') + (auto ? "++" : "") + nameFromKeyPath(keyPath)
	        };
	    }
	    function nameFromKeyPath(keyPath) {
	        return typeof keyPath === 'string' ?
	            keyPath :
	            keyPath ? ('[' + [].join.call(keyPath, '+') + ']') : "";
	    }

	    function createTableSchema(name, primKey, indexes) {
	        return {
	            name: name,
	            primKey: primKey,
	            indexes: indexes,
	            mappedClass: null,
	            idxByName: arrayToObject(indexes, function (index) { return [index.name, index]; })
	        };
	    }

	    function safariMultiStoreFix(storeNames) {
	        return storeNames.length === 1 ? storeNames[0] : storeNames;
	    }
	    var getMaxKey = function (IdbKeyRange) {
	        try {
	            IdbKeyRange.only([[]]);
	            getMaxKey = function () { return [[]]; };
	            return [[]];
	        }
	        catch (e) {
	            getMaxKey = function () { return maxString; };
	            return maxString;
	        }
	    };

	    function getKeyExtractor(keyPath) {
	        if (keyPath == null) {
	            return function () { return undefined; };
	        }
	        else if (typeof keyPath === 'string') {
	            return getSinglePathKeyExtractor(keyPath);
	        }
	        else {
	            return function (obj) { return getByKeyPath(obj, keyPath); };
	        }
	    }
	    function getSinglePathKeyExtractor(keyPath) {
	        var split = keyPath.split('.');
	        if (split.length === 1) {
	            return function (obj) { return obj[keyPath]; };
	        }
	        else {
	            return function (obj) { return getByKeyPath(obj, keyPath); };
	        }
	    }

	    function arrayify(arrayLike) {
	        return [].slice.call(arrayLike);
	    }
	    var _id_counter = 0;
	    function getKeyPathAlias(keyPath) {
	        return keyPath == null ?
	            ":id" :
	            typeof keyPath === 'string' ?
	                keyPath :
	                "[".concat(keyPath.join('+'), "]");
	    }
	    function createDBCore(db, IdbKeyRange, tmpTrans) {
	        function extractSchema(db, trans) {
	            var tables = arrayify(db.objectStoreNames);
	            return {
	                schema: {
	                    name: db.name,
	                    tables: tables.map(function (table) { return trans.objectStore(table); }).map(function (store) {
	                        var keyPath = store.keyPath, autoIncrement = store.autoIncrement;
	                        var compound = isArray(keyPath);
	                        var outbound = keyPath == null;
	                        var indexByKeyPath = {};
	                        var result = {
	                            name: store.name,
	                            primaryKey: {
	                                name: null,
	                                isPrimaryKey: true,
	                                outbound: outbound,
	                                compound: compound,
	                                keyPath: keyPath,
	                                autoIncrement: autoIncrement,
	                                unique: true,
	                                extractKey: getKeyExtractor(keyPath)
	                            },
	                            indexes: arrayify(store.indexNames).map(function (indexName) { return store.index(indexName); })
	                                .map(function (index) {
	                                var name = index.name, unique = index.unique, multiEntry = index.multiEntry, keyPath = index.keyPath;
	                                var compound = isArray(keyPath);
	                                var result = {
	                                    name: name,
	                                    compound: compound,
	                                    keyPath: keyPath,
	                                    unique: unique,
	                                    multiEntry: multiEntry,
	                                    extractKey: getKeyExtractor(keyPath)
	                                };
	                                indexByKeyPath[getKeyPathAlias(keyPath)] = result;
	                                return result;
	                            }),
	                            getIndexByKeyPath: function (keyPath) { return indexByKeyPath[getKeyPathAlias(keyPath)]; }
	                        };
	                        indexByKeyPath[":id"] = result.primaryKey;
	                        if (keyPath != null) {
	                            indexByKeyPath[getKeyPathAlias(keyPath)] = result.primaryKey;
	                        }
	                        return result;
	                    })
	                },
	                hasGetAll: tables.length > 0 && ('getAll' in trans.objectStore(tables[0])) &&
	                    !(typeof navigator !== 'undefined' && /Safari/.test(navigator.userAgent) &&
	                        !/(Chrome\/|Edge\/)/.test(navigator.userAgent) &&
	                        [].concat(navigator.userAgent.match(/Safari\/(\d*)/))[1] < 604)
	            };
	        }
	        function makeIDBKeyRange(range) {
	            if (range.type === 3 )
	                return null;
	            if (range.type === 4 )
	                throw new Error("Cannot convert never type to IDBKeyRange");
	            var lower = range.lower, upper = range.upper, lowerOpen = range.lowerOpen, upperOpen = range.upperOpen;
	            var idbRange = lower === undefined ?
	                upper === undefined ?
	                    null :
	                    IdbKeyRange.upperBound(upper, !!upperOpen) :
	                upper === undefined ?
	                    IdbKeyRange.lowerBound(lower, !!lowerOpen) :
	                    IdbKeyRange.bound(lower, upper, !!lowerOpen, !!upperOpen);
	            return idbRange;
	        }
	        function createDbCoreTable(tableSchema) {
	            var tableName = tableSchema.name;
	            function mutate(_a) {
	                var trans = _a.trans, type = _a.type, keys = _a.keys, values = _a.values, range = _a.range;
	                return new Promise(function (resolve, reject) {
	                    resolve = wrap(resolve);
	                    var store = trans.objectStore(tableName);
	                    var outbound = store.keyPath == null;
	                    var isAddOrPut = type === "put" || type === "add";
	                    if (!isAddOrPut && type !== 'delete' && type !== 'deleteRange')
	                        throw new Error("Invalid operation type: " + type);
	                    var length = (keys || values || { length: 1 }).length;
	                    if (keys && values && keys.length !== values.length) {
	                        throw new Error("Given keys array must have same length as given values array.");
	                    }
	                    if (length === 0)
	                        return resolve({ numFailures: 0, failures: {}, results: [], lastResult: undefined });
	                    var req;
	                    var reqs = [];
	                    var failures = [];
	                    var numFailures = 0;
	                    var errorHandler = function (event) {
	                        ++numFailures;
	                        preventDefault(event);
	                    };
	                    if (type === 'deleteRange') {
	                        if (range.type === 4 )
	                            return resolve({ numFailures: numFailures, failures: failures, results: [], lastResult: undefined });
	                        if (range.type === 3 )
	                            reqs.push(req = store.clear());
	                        else
	                            reqs.push(req = store.delete(makeIDBKeyRange(range)));
	                    }
	                    else {
	                        var _a = isAddOrPut ?
	                            outbound ?
	                                [values, keys] :
	                                [values, null] :
	                            [keys, null], args1 = _a[0], args2 = _a[1];
	                        if (isAddOrPut) {
	                            for (var i = 0; i < length; ++i) {
	                                reqs.push(req = (args2 && args2[i] !== undefined ?
	                                    store[type](args1[i], args2[i]) :
	                                    store[type](args1[i])));
	                                req.onerror = errorHandler;
	                            }
	                        }
	                        else {
	                            for (var i = 0; i < length; ++i) {
	                                reqs.push(req = store[type](args1[i]));
	                                req.onerror = errorHandler;
	                            }
	                        }
	                    }
	                    var done = function (event) {
	                        var lastResult = event.target.result;
	                        reqs.forEach(function (req, i) { return req.error != null && (failures[i] = req.error); });
	                        resolve({
	                            numFailures: numFailures,
	                            failures: failures,
	                            results: type === "delete" ? keys : reqs.map(function (req) { return req.result; }),
	                            lastResult: lastResult
	                        });
	                    };
	                    req.onerror = function (event) {
	                        errorHandler(event);
	                        done(event);
	                    };
	                    req.onsuccess = done;
	                });
	            }
	            function openCursor(_a) {
	                var trans = _a.trans, values = _a.values, query = _a.query, reverse = _a.reverse, unique = _a.unique;
	                return new Promise(function (resolve, reject) {
	                    resolve = wrap(resolve);
	                    var index = query.index, range = query.range;
	                    var store = trans.objectStore(tableName);
	                    var source = index.isPrimaryKey ?
	                        store :
	                        store.index(index.name);
	                    var direction = reverse ?
	                        unique ?
	                            "prevunique" :
	                            "prev" :
	                        unique ?
	                            "nextunique" :
	                            "next";
	                    var req = values || !('openKeyCursor' in source) ?
	                        source.openCursor(makeIDBKeyRange(range), direction) :
	                        source.openKeyCursor(makeIDBKeyRange(range), direction);
	                    req.onerror = eventRejectHandler(reject);
	                    req.onsuccess = wrap(function (ev) {
	                        var cursor = req.result;
	                        if (!cursor) {
	                            resolve(null);
	                            return;
	                        }
	                        cursor.___id = ++_id_counter;
	                        cursor.done = false;
	                        var _cursorContinue = cursor.continue.bind(cursor);
	                        var _cursorContinuePrimaryKey = cursor.continuePrimaryKey;
	                        if (_cursorContinuePrimaryKey)
	                            _cursorContinuePrimaryKey = _cursorContinuePrimaryKey.bind(cursor);
	                        var _cursorAdvance = cursor.advance.bind(cursor);
	                        var doThrowCursorIsNotStarted = function () { throw new Error("Cursor not started"); };
	                        var doThrowCursorIsStopped = function () { throw new Error("Cursor not stopped"); };
	                        cursor.trans = trans;
	                        cursor.stop = cursor.continue = cursor.continuePrimaryKey = cursor.advance = doThrowCursorIsNotStarted;
	                        cursor.fail = wrap(reject);
	                        cursor.next = function () {
	                            var _this = this;
	                            var gotOne = 1;
	                            return this.start(function () { return gotOne-- ? _this.continue() : _this.stop(); }).then(function () { return _this; });
	                        };
	                        cursor.start = function (callback) {
	                            var iterationPromise = new Promise(function (resolveIteration, rejectIteration) {
	                                resolveIteration = wrap(resolveIteration);
	                                req.onerror = eventRejectHandler(rejectIteration);
	                                cursor.fail = rejectIteration;
	                                cursor.stop = function (value) {
	                                    cursor.stop = cursor.continue = cursor.continuePrimaryKey = cursor.advance = doThrowCursorIsStopped;
	                                    resolveIteration(value);
	                                };
	                            });
	                            var guardedCallback = function () {
	                                if (req.result) {
	                                    try {
	                                        callback();
	                                    }
	                                    catch (err) {
	                                        cursor.fail(err);
	                                    }
	                                }
	                                else {
	                                    cursor.done = true;
	                                    cursor.start = function () { throw new Error("Cursor behind last entry"); };
	                                    cursor.stop();
	                                }
	                            };
	                            req.onsuccess = wrap(function (ev) {
	                                req.onsuccess = guardedCallback;
	                                guardedCallback();
	                            });
	                            cursor.continue = _cursorContinue;
	                            cursor.continuePrimaryKey = _cursorContinuePrimaryKey;
	                            cursor.advance = _cursorAdvance;
	                            guardedCallback();
	                            return iterationPromise;
	                        };
	                        resolve(cursor);
	                    }, reject);
	                });
	            }
	            function query(hasGetAll) {
	                return function (request) {
	                    return new Promise(function (resolve, reject) {
	                        resolve = wrap(resolve);
	                        var trans = request.trans, values = request.values, limit = request.limit, query = request.query;
	                        var nonInfinitLimit = limit === Infinity ? undefined : limit;
	                        var index = query.index, range = query.range;
	                        var store = trans.objectStore(tableName);
	                        var source = index.isPrimaryKey ? store : store.index(index.name);
	                        var idbKeyRange = makeIDBKeyRange(range);
	                        if (limit === 0)
	                            return resolve({ result: [] });
	                        if (hasGetAll) {
	                            var req = values ?
	                                source.getAll(idbKeyRange, nonInfinitLimit) :
	                                source.getAllKeys(idbKeyRange, nonInfinitLimit);
	                            req.onsuccess = function (event) { return resolve({ result: event.target.result }); };
	                            req.onerror = eventRejectHandler(reject);
	                        }
	                        else {
	                            var count_1 = 0;
	                            var req_1 = values || !('openKeyCursor' in source) ?
	                                source.openCursor(idbKeyRange) :
	                                source.openKeyCursor(idbKeyRange);
	                            var result_1 = [];
	                            req_1.onsuccess = function (event) {
	                                var cursor = req_1.result;
	                                if (!cursor)
	                                    return resolve({ result: result_1 });
	                                result_1.push(values ? cursor.value : cursor.primaryKey);
	                                if (++count_1 === limit)
	                                    return resolve({ result: result_1 });
	                                cursor.continue();
	                            };
	                            req_1.onerror = eventRejectHandler(reject);
	                        }
	                    });
	                };
	            }
	            return {
	                name: tableName,
	                schema: tableSchema,
	                mutate: mutate,
	                getMany: function (_a) {
	                    var trans = _a.trans, keys = _a.keys;
	                    return new Promise(function (resolve, reject) {
	                        resolve = wrap(resolve);
	                        var store = trans.objectStore(tableName);
	                        var length = keys.length;
	                        var result = new Array(length);
	                        var keyCount = 0;
	                        var callbackCount = 0;
	                        var req;
	                        var successHandler = function (event) {
	                            var req = event.target;
	                            if ((result[req._pos] = req.result) != null)
	                                ;
	                            if (++callbackCount === keyCount)
	                                resolve(result);
	                        };
	                        var errorHandler = eventRejectHandler(reject);
	                        for (var i = 0; i < length; ++i) {
	                            var key = keys[i];
	                            if (key != null) {
	                                req = store.get(keys[i]);
	                                req._pos = i;
	                                req.onsuccess = successHandler;
	                                req.onerror = errorHandler;
	                                ++keyCount;
	                            }
	                        }
	                        if (keyCount === 0)
	                            resolve(result);
	                    });
	                },
	                get: function (_a) {
	                    var trans = _a.trans, key = _a.key;
	                    return new Promise(function (resolve, reject) {
	                        resolve = wrap(resolve);
	                        var store = trans.objectStore(tableName);
	                        var req = store.get(key);
	                        req.onsuccess = function (event) { return resolve(event.target.result); };
	                        req.onerror = eventRejectHandler(reject);
	                    });
	                },
	                query: query(hasGetAll),
	                openCursor: openCursor,
	                count: function (_a) {
	                    var query = _a.query, trans = _a.trans;
	                    var index = query.index, range = query.range;
	                    return new Promise(function (resolve, reject) {
	                        var store = trans.objectStore(tableName);
	                        var source = index.isPrimaryKey ? store : store.index(index.name);
	                        var idbKeyRange = makeIDBKeyRange(range);
	                        var req = idbKeyRange ? source.count(idbKeyRange) : source.count();
	                        req.onsuccess = wrap(function (ev) { return resolve(ev.target.result); });
	                        req.onerror = eventRejectHandler(reject);
	                    });
	                }
	            };
	        }
	        var _a = extractSchema(db, tmpTrans), schema = _a.schema, hasGetAll = _a.hasGetAll;
	        var tables = schema.tables.map(function (tableSchema) { return createDbCoreTable(tableSchema); });
	        var tableMap = {};
	        tables.forEach(function (table) { return tableMap[table.name] = table; });
	        return {
	            stack: "dbcore",
	            transaction: db.transaction.bind(db),
	            table: function (name) {
	                var result = tableMap[name];
	                if (!result)
	                    throw new Error("Table '".concat(name, "' not found"));
	                return tableMap[name];
	            },
	            MIN_KEY: -Infinity,
	            MAX_KEY: getMaxKey(IdbKeyRange),
	            schema: schema
	        };
	    }

	    function createMiddlewareStack(stackImpl, middlewares) {
	        return middlewares.reduce(function (down, _a) {
	            var create = _a.create;
	            return (__assign(__assign({}, down), create(down)));
	        }, stackImpl);
	    }
	    function createMiddlewareStacks(middlewares, idbdb, _a, tmpTrans) {
	        var IDBKeyRange = _a.IDBKeyRange; _a.indexedDB;
	        var dbcore = createMiddlewareStack(createDBCore(idbdb, IDBKeyRange, tmpTrans), middlewares.dbcore);
	        return {
	            dbcore: dbcore
	        };
	    }
	    function generateMiddlewareStacks(db, tmpTrans) {
	        var idbdb = tmpTrans.db;
	        var stacks = createMiddlewareStacks(db._middlewares, idbdb, db._deps, tmpTrans);
	        db.core = stacks.dbcore;
	        db.tables.forEach(function (table) {
	            var tableName = table.name;
	            if (db.core.schema.tables.some(function (tbl) { return tbl.name === tableName; })) {
	                table.core = db.core.table(tableName);
	                if (db[tableName] instanceof db.Table) {
	                    db[tableName].core = table.core;
	                }
	            }
	        });
	    }

	    function setApiOnPlace(db, objs, tableNames, dbschema) {
	        tableNames.forEach(function (tableName) {
	            var schema = dbschema[tableName];
	            objs.forEach(function (obj) {
	                var propDesc = getPropertyDescriptor(obj, tableName);
	                if (!propDesc || ("value" in propDesc && propDesc.value === undefined)) {
	                    if (obj === db.Transaction.prototype || obj instanceof db.Transaction) {
	                        setProp(obj, tableName, {
	                            get: function () { return this.table(tableName); },
	                            set: function (value) {
	                                defineProperty(this, tableName, { value: value, writable: true, configurable: true, enumerable: true });
	                            }
	                        });
	                    }
	                    else {
	                        obj[tableName] = new db.Table(tableName, schema);
	                    }
	                }
	            });
	        });
	    }
	    function removeTablesApi(db, objs) {
	        objs.forEach(function (obj) {
	            for (var key in obj) {
	                if (obj[key] instanceof db.Table)
	                    delete obj[key];
	            }
	        });
	    }
	    function lowerVersionFirst(a, b) {
	        return a._cfg.version - b._cfg.version;
	    }
	    function runUpgraders(db, oldVersion, idbUpgradeTrans, reject) {
	        var globalSchema = db._dbSchema;
	        if (idbUpgradeTrans.objectStoreNames.contains('$meta') && !globalSchema.$meta) {
	            globalSchema.$meta = createTableSchema("$meta", parseIndexSyntax("")[0], []);
	            db._storeNames.push('$meta');
	        }
	        var trans = db._createTransaction('readwrite', db._storeNames, globalSchema);
	        trans.create(idbUpgradeTrans);
	        trans._completion.catch(reject);
	        var rejectTransaction = trans._reject.bind(trans);
	        var transless = PSD.transless || PSD;
	        newScope(function () {
	            PSD.trans = trans;
	            PSD.transless = transless;
	            if (oldVersion === 0) {
	                keys(globalSchema).forEach(function (tableName) {
	                    createTable(idbUpgradeTrans, tableName, globalSchema[tableName].primKey, globalSchema[tableName].indexes);
	                });
	                generateMiddlewareStacks(db, idbUpgradeTrans);
	                DexiePromise.follow(function () { return db.on.populate.fire(trans); }).catch(rejectTransaction);
	            }
	            else {
	                generateMiddlewareStacks(db, idbUpgradeTrans);
	                return getExistingVersion(db, trans, oldVersion)
	                    .then(function (oldVersion) { return updateTablesAndIndexes(db, oldVersion, trans, idbUpgradeTrans); })
	                    .catch(rejectTransaction);
	            }
	        });
	    }
	    function patchCurrentVersion(db, idbUpgradeTrans) {
	        createMissingTables(db._dbSchema, idbUpgradeTrans);
	        if (idbUpgradeTrans.db.version % 10 === 0 && !idbUpgradeTrans.objectStoreNames.contains('$meta')) {
	            idbUpgradeTrans.db.createObjectStore('$meta').add(Math.ceil((idbUpgradeTrans.db.version / 10) - 1), 'version');
	        }
	        var globalSchema = buildGlobalSchema(db, db.idbdb, idbUpgradeTrans);
	        adjustToExistingIndexNames(db, db._dbSchema, idbUpgradeTrans);
	        var diff = getSchemaDiff(globalSchema, db._dbSchema);
	        var _loop_1 = function (tableChange) {
	            if (tableChange.change.length || tableChange.recreate) {
	                console.warn("Unable to patch indexes of table ".concat(tableChange.name, " because it has changes on the type of index or primary key."));
	                return { value: void 0 };
	            }
	            var store = idbUpgradeTrans.objectStore(tableChange.name);
	            tableChange.add.forEach(function (idx) {
	                if (debug)
	                    console.debug("Dexie upgrade patch: Creating missing index ".concat(tableChange.name, ".").concat(idx.src));
	                addIndex(store, idx);
	            });
	        };
	        for (var _i = 0, _a = diff.change; _i < _a.length; _i++) {
	            var tableChange = _a[_i];
	            var state_1 = _loop_1(tableChange);
	            if (typeof state_1 === "object")
	                return state_1.value;
	        }
	    }
	    function getExistingVersion(db, trans, oldVersion) {
	        if (trans.storeNames.includes('$meta')) {
	            return trans.table('$meta').get('version').then(function (metaVersion) {
	                return metaVersion != null ? metaVersion : oldVersion;
	            });
	        }
	        else {
	            return DexiePromise.resolve(oldVersion);
	        }
	    }
	    function updateTablesAndIndexes(db, oldVersion, trans, idbUpgradeTrans) {
	        var queue = [];
	        var versions = db._versions;
	        var globalSchema = db._dbSchema = buildGlobalSchema(db, db.idbdb, idbUpgradeTrans);
	        var versToRun = versions.filter(function (v) { return v._cfg.version >= oldVersion; });
	        if (versToRun.length === 0) {
	            return DexiePromise.resolve();
	        }
	        versToRun.forEach(function (version) {
	            queue.push(function () {
	                var oldSchema = globalSchema;
	                var newSchema = version._cfg.dbschema;
	                adjustToExistingIndexNames(db, oldSchema, idbUpgradeTrans);
	                adjustToExistingIndexNames(db, newSchema, idbUpgradeTrans);
	                globalSchema = db._dbSchema = newSchema;
	                var diff = getSchemaDiff(oldSchema, newSchema);
	                diff.add.forEach(function (tuple) {
	                    createTable(idbUpgradeTrans, tuple[0], tuple[1].primKey, tuple[1].indexes);
	                });
	                diff.change.forEach(function (change) {
	                    if (change.recreate) {
	                        throw new exceptions.Upgrade("Not yet support for changing primary key");
	                    }
	                    else {
	                        var store_1 = idbUpgradeTrans.objectStore(change.name);
	                        change.add.forEach(function (idx) { return addIndex(store_1, idx); });
	                        change.change.forEach(function (idx) {
	                            store_1.deleteIndex(idx.name);
	                            addIndex(store_1, idx);
	                        });
	                        change.del.forEach(function (idxName) { return store_1.deleteIndex(idxName); });
	                    }
	                });
	                var contentUpgrade = version._cfg.contentUpgrade;
	                if (contentUpgrade && version._cfg.version > oldVersion) {
	                    generateMiddlewareStacks(db, idbUpgradeTrans);
	                    trans._memoizedTables = {};
	                    var upgradeSchema_1 = shallowClone(newSchema);
	                    diff.del.forEach(function (table) {
	                        upgradeSchema_1[table] = oldSchema[table];
	                    });
	                    removeTablesApi(db, [db.Transaction.prototype]);
	                    setApiOnPlace(db, [db.Transaction.prototype], keys(upgradeSchema_1), upgradeSchema_1);
	                    trans.schema = upgradeSchema_1;
	                    var contentUpgradeIsAsync_1 = isAsyncFunction(contentUpgrade);
	                    if (contentUpgradeIsAsync_1) {
	                        incrementExpectedAwaits();
	                    }
	                    var returnValue_1;
	                    var promiseFollowed = DexiePromise.follow(function () {
	                        returnValue_1 = contentUpgrade(trans);
	                        if (returnValue_1) {
	                            if (contentUpgradeIsAsync_1) {
	                                var decrementor = decrementExpectedAwaits.bind(null, null);
	                                returnValue_1.then(decrementor, decrementor);
	                            }
	                        }
	                    });
	                    return (returnValue_1 && typeof returnValue_1.then === 'function' ?
	                        DexiePromise.resolve(returnValue_1) : promiseFollowed.then(function () { return returnValue_1; }));
	                }
	            });
	            queue.push(function (idbtrans) {
	                var newSchema = version._cfg.dbschema;
	                deleteRemovedTables(newSchema, idbtrans);
	                removeTablesApi(db, [db.Transaction.prototype]);
	                setApiOnPlace(db, [db.Transaction.prototype], db._storeNames, db._dbSchema);
	                trans.schema = db._dbSchema;
	            });
	            queue.push(function (idbtrans) {
	                if (db.idbdb.objectStoreNames.contains('$meta')) {
	                    if (Math.ceil(db.idbdb.version / 10) === version._cfg.version) {
	                        db.idbdb.deleteObjectStore('$meta');
	                        delete db._dbSchema.$meta;
	                        db._storeNames = db._storeNames.filter(function (name) { return name !== '$meta'; });
	                    }
	                    else {
	                        idbtrans.objectStore('$meta').put(version._cfg.version, 'version');
	                    }
	                }
	            });
	        });
	        function runQueue() {
	            return queue.length ? DexiePromise.resolve(queue.shift()(trans.idbtrans)).then(runQueue) :
	                DexiePromise.resolve();
	        }
	        return runQueue().then(function () {
	            createMissingTables(globalSchema, idbUpgradeTrans);
	        });
	    }
	    function getSchemaDiff(oldSchema, newSchema) {
	        var diff = {
	            del: [],
	            add: [],
	            change: []
	        };
	        var table;
	        for (table in oldSchema) {
	            if (!newSchema[table])
	                diff.del.push(table);
	        }
	        for (table in newSchema) {
	            var oldDef = oldSchema[table], newDef = newSchema[table];
	            if (!oldDef) {
	                diff.add.push([table, newDef]);
	            }
	            else {
	                var change = {
	                    name: table,
	                    def: newDef,
	                    recreate: false,
	                    del: [],
	                    add: [],
	                    change: []
	                };
	                if ((
	                '' + (oldDef.primKey.keyPath || '')) !== ('' + (newDef.primKey.keyPath || '')) ||
	                    (oldDef.primKey.auto !== newDef.primKey.auto)) {
	                    change.recreate = true;
	                    diff.change.push(change);
	                }
	                else {
	                    var oldIndexes = oldDef.idxByName;
	                    var newIndexes = newDef.idxByName;
	                    var idxName = void 0;
	                    for (idxName in oldIndexes) {
	                        if (!newIndexes[idxName])
	                            change.del.push(idxName);
	                    }
	                    for (idxName in newIndexes) {
	                        var oldIdx = oldIndexes[idxName], newIdx = newIndexes[idxName];
	                        if (!oldIdx)
	                            change.add.push(newIdx);
	                        else if (oldIdx.src !== newIdx.src)
	                            change.change.push(newIdx);
	                    }
	                    if (change.del.length > 0 || change.add.length > 0 || change.change.length > 0) {
	                        diff.change.push(change);
	                    }
	                }
	            }
	        }
	        return diff;
	    }
	    function createTable(idbtrans, tableName, primKey, indexes) {
	        var store = idbtrans.db.createObjectStore(tableName, primKey.keyPath ?
	            { keyPath: primKey.keyPath, autoIncrement: primKey.auto } :
	            { autoIncrement: primKey.auto });
	        indexes.forEach(function (idx) { return addIndex(store, idx); });
	        return store;
	    }
	    function createMissingTables(newSchema, idbtrans) {
	        keys(newSchema).forEach(function (tableName) {
	            if (!idbtrans.db.objectStoreNames.contains(tableName)) {
	                if (debug)
	                    console.debug('Dexie: Creating missing table', tableName);
	                createTable(idbtrans, tableName, newSchema[tableName].primKey, newSchema[tableName].indexes);
	            }
	        });
	    }
	    function deleteRemovedTables(newSchema, idbtrans) {
	        [].slice.call(idbtrans.db.objectStoreNames).forEach(function (storeName) {
	            return newSchema[storeName] == null && idbtrans.db.deleteObjectStore(storeName);
	        });
	    }
	    function addIndex(store, idx) {
	        store.createIndex(idx.name, idx.keyPath, { unique: idx.unique, multiEntry: idx.multi });
	    }
	    function buildGlobalSchema(db, idbdb, tmpTrans) {
	        var globalSchema = {};
	        var dbStoreNames = slice(idbdb.objectStoreNames, 0);
	        dbStoreNames.forEach(function (storeName) {
	            var store = tmpTrans.objectStore(storeName);
	            var keyPath = store.keyPath;
	            var primKey = createIndexSpec(nameFromKeyPath(keyPath), keyPath || "", true, false, !!store.autoIncrement, keyPath && typeof keyPath !== "string", true);
	            var indexes = [];
	            for (var j = 0; j < store.indexNames.length; ++j) {
	                var idbindex = store.index(store.indexNames[j]);
	                keyPath = idbindex.keyPath;
	                var index = createIndexSpec(idbindex.name, keyPath, !!idbindex.unique, !!idbindex.multiEntry, false, keyPath && typeof keyPath !== "string", false);
	                indexes.push(index);
	            }
	            globalSchema[storeName] = createTableSchema(storeName, primKey, indexes);
	        });
	        return globalSchema;
	    }
	    function readGlobalSchema(db, idbdb, tmpTrans) {
	        db.verno = idbdb.version / 10;
	        var globalSchema = db._dbSchema = buildGlobalSchema(db, idbdb, tmpTrans);
	        db._storeNames = slice(idbdb.objectStoreNames, 0);
	        setApiOnPlace(db, [db._allTables], keys(globalSchema), globalSchema);
	    }
	    function verifyInstalledSchema(db, tmpTrans) {
	        var installedSchema = buildGlobalSchema(db, db.idbdb, tmpTrans);
	        var diff = getSchemaDiff(installedSchema, db._dbSchema);
	        return !(diff.add.length || diff.change.some(function (ch) { return ch.add.length || ch.change.length; }));
	    }
	    function adjustToExistingIndexNames(db, schema, idbtrans) {
	        var storeNames = idbtrans.db.objectStoreNames;
	        for (var i = 0; i < storeNames.length; ++i) {
	            var storeName = storeNames[i];
	            var store = idbtrans.objectStore(storeName);
	            db._hasGetAll = 'getAll' in store;
	            for (var j = 0; j < store.indexNames.length; ++j) {
	                var indexName = store.indexNames[j];
	                var keyPath = store.index(indexName).keyPath;
	                var dexieName = typeof keyPath === 'string' ? keyPath : "[" + slice(keyPath).join('+') + "]";
	                if (schema[storeName]) {
	                    var indexSpec = schema[storeName].idxByName[dexieName];
	                    if (indexSpec) {
	                        indexSpec.name = indexName;
	                        delete schema[storeName].idxByName[dexieName];
	                        schema[storeName].idxByName[indexName] = indexSpec;
	                    }
	                }
	            }
	        }
	        if (typeof navigator !== 'undefined' && /Safari/.test(navigator.userAgent) &&
	            !/(Chrome\/|Edge\/)/.test(navigator.userAgent) &&
	            _global.WorkerGlobalScope && _global instanceof _global.WorkerGlobalScope &&
	            [].concat(navigator.userAgent.match(/Safari\/(\d*)/))[1] < 604) {
	            db._hasGetAll = false;
	        }
	    }
	    function parseIndexSyntax(primKeyAndIndexes) {
	        return primKeyAndIndexes.split(',').map(function (index, indexNum) {
	            index = index.trim();
	            var name = index.replace(/([&*]|\+\+)/g, "");
	            var keyPath = /^\[/.test(name) ? name.match(/^\[(.*)\]$/)[1].split('+') : name;
	            return createIndexSpec(name, keyPath || null, /\&/.test(index), /\*/.test(index), /\+\+/.test(index), isArray(keyPath), indexNum === 0);
	        });
	    }

	    var Version =  (function () {
	        function Version() {
	        }
	        Version.prototype._parseStoresSpec = function (stores, outSchema) {
	            keys(stores).forEach(function (tableName) {
	                if (stores[tableName] !== null) {
	                    var indexes = parseIndexSyntax(stores[tableName]);
	                    var primKey = indexes.shift();
	                    primKey.unique = true;
	                    if (primKey.multi)
	                        throw new exceptions.Schema("Primary key cannot be multi-valued");
	                    indexes.forEach(function (idx) {
	                        if (idx.auto)
	                            throw new exceptions.Schema("Only primary key can be marked as autoIncrement (++)");
	                        if (!idx.keyPath)
	                            throw new exceptions.Schema("Index must have a name and cannot be an empty string");
	                    });
	                    outSchema[tableName] = createTableSchema(tableName, primKey, indexes);
	                }
	            });
	        };
	        Version.prototype.stores = function (stores) {
	            var db = this.db;
	            this._cfg.storesSource = this._cfg.storesSource ?
	                extend(this._cfg.storesSource, stores) :
	                stores;
	            var versions = db._versions;
	            var storesSpec = {};
	            var dbschema = {};
	            versions.forEach(function (version) {
	                extend(storesSpec, version._cfg.storesSource);
	                dbschema = (version._cfg.dbschema = {});
	                version._parseStoresSpec(storesSpec, dbschema);
	            });
	            db._dbSchema = dbschema;
	            removeTablesApi(db, [db._allTables, db, db.Transaction.prototype]);
	            setApiOnPlace(db, [db._allTables, db, db.Transaction.prototype, this._cfg.tables], keys(dbschema), dbschema);
	            db._storeNames = keys(dbschema);
	            return this;
	        };
	        Version.prototype.upgrade = function (upgradeFunction) {
	            this._cfg.contentUpgrade = promisableChain(this._cfg.contentUpgrade || nop, upgradeFunction);
	            return this;
	        };
	        return Version;
	    }());

	    function createVersionConstructor(db) {
	        return makeClassConstructor(Version.prototype, function Version(versionNumber) {
	            this.db = db;
	            this._cfg = {
	                version: versionNumber,
	                storesSource: null,
	                dbschema: {},
	                tables: {},
	                contentUpgrade: null
	            };
	        });
	    }

	    function getDbNamesTable(indexedDB, IDBKeyRange) {
	        var dbNamesDB = indexedDB["_dbNamesDB"];
	        if (!dbNamesDB) {
	            dbNamesDB = indexedDB["_dbNamesDB"] = new Dexie$1(DBNAMES_DB, {
	                addons: [],
	                indexedDB: indexedDB,
	                IDBKeyRange: IDBKeyRange,
	            });
	            dbNamesDB.version(1).stores({ dbnames: "name" });
	        }
	        return dbNamesDB.table("dbnames");
	    }
	    function hasDatabasesNative(indexedDB) {
	        return indexedDB && typeof indexedDB.databases === "function";
	    }
	    function getDatabaseNames(_a) {
	        var indexedDB = _a.indexedDB, IDBKeyRange = _a.IDBKeyRange;
	        return hasDatabasesNative(indexedDB)
	            ? Promise.resolve(indexedDB.databases()).then(function (infos) {
	                return infos
	                    .map(function (info) { return info.name; })
	                    .filter(function (name) { return name !== DBNAMES_DB; });
	            })
	            : getDbNamesTable(indexedDB, IDBKeyRange).toCollection().primaryKeys();
	    }
	    function _onDatabaseCreated(_a, name) {
	        var indexedDB = _a.indexedDB, IDBKeyRange = _a.IDBKeyRange;
	        !hasDatabasesNative(indexedDB) &&
	            name !== DBNAMES_DB &&
	            getDbNamesTable(indexedDB, IDBKeyRange).put({ name: name }).catch(nop);
	    }
	    function _onDatabaseDeleted(_a, name) {
	        var indexedDB = _a.indexedDB, IDBKeyRange = _a.IDBKeyRange;
	        !hasDatabasesNative(indexedDB) &&
	            name !== DBNAMES_DB &&
	            getDbNamesTable(indexedDB, IDBKeyRange).delete(name).catch(nop);
	    }

	    function vip(fn) {
	        return newScope(function () {
	            PSD.letThrough = true;
	            return fn();
	        });
	    }

	    function idbReady() {
	        var isSafari = !navigator.userAgentData &&
	            /Safari\//.test(navigator.userAgent) &&
	            !/Chrom(e|ium)\//.test(navigator.userAgent);
	        if (!isSafari || !indexedDB.databases)
	            return Promise.resolve();
	        var intervalId;
	        return new Promise(function (resolve) {
	            var tryIdb = function () { return indexedDB.databases().finally(resolve); };
	            intervalId = setInterval(tryIdb, 100);
	            tryIdb();
	        }).finally(function () { return clearInterval(intervalId); });
	    }

	    var _a;
	    function isEmptyRange(node) {
	        return !("from" in node);
	    }
	    var RangeSet = function (fromOrTree, to) {
	        if (this) {
	            extend(this, arguments.length ? { d: 1, from: fromOrTree, to: arguments.length > 1 ? to : fromOrTree } : { d: 0 });
	        }
	        else {
	            var rv = new RangeSet();
	            if (fromOrTree && ("d" in fromOrTree)) {
	                extend(rv, fromOrTree);
	            }
	            return rv;
	        }
	    };
	    props(RangeSet.prototype, (_a = {
	            add: function (rangeSet) {
	                mergeRanges(this, rangeSet);
	                return this;
	            },
	            addKey: function (key) {
	                addRange(this, key, key);
	                return this;
	            },
	            addKeys: function (keys) {
	                var _this = this;
	                keys.forEach(function (key) { return addRange(_this, key, key); });
	                return this;
	            }
	        },
	        _a[iteratorSymbol] = function () {
	            return getRangeSetIterator(this);
	        },
	        _a));
	    function addRange(target, from, to) {
	        var diff = cmp(from, to);
	        if (isNaN(diff))
	            return;
	        if (diff > 0)
	            throw RangeError();
	        if (isEmptyRange(target))
	            return extend(target, { from: from, to: to, d: 1 });
	        var left = target.l;
	        var right = target.r;
	        if (cmp(to, target.from) < 0) {
	            left
	                ? addRange(left, from, to)
	                : (target.l = { from: from, to: to, d: 1, l: null, r: null });
	            return rebalance(target);
	        }
	        if (cmp(from, target.to) > 0) {
	            right
	                ? addRange(right, from, to)
	                : (target.r = { from: from, to: to, d: 1, l: null, r: null });
	            return rebalance(target);
	        }
	        if (cmp(from, target.from) < 0) {
	            target.from = from;
	            target.l = null;
	            target.d = right ? right.d + 1 : 1;
	        }
	        if (cmp(to, target.to) > 0) {
	            target.to = to;
	            target.r = null;
	            target.d = target.l ? target.l.d + 1 : 1;
	        }
	        var rightWasCutOff = !target.r;
	        if (left && !target.l) {
	            mergeRanges(target, left);
	        }
	        if (right && rightWasCutOff) {
	            mergeRanges(target, right);
	        }
	    }
	    function mergeRanges(target, newSet) {
	        function _addRangeSet(target, _a) {
	            var from = _a.from, to = _a.to, l = _a.l, r = _a.r;
	            addRange(target, from, to);
	            if (l)
	                _addRangeSet(target, l);
	            if (r)
	                _addRangeSet(target, r);
	        }
	        if (!isEmptyRange(newSet))
	            _addRangeSet(target, newSet);
	    }
	    function rangesOverlap(rangeSet1, rangeSet2) {
	        var i1 = getRangeSetIterator(rangeSet2);
	        var nextResult1 = i1.next();
	        if (nextResult1.done)
	            return false;
	        var a = nextResult1.value;
	        var i2 = getRangeSetIterator(rangeSet1);
	        var nextResult2 = i2.next(a.from);
	        var b = nextResult2.value;
	        while (!nextResult1.done && !nextResult2.done) {
	            if (cmp(b.from, a.to) <= 0 && cmp(b.to, a.from) >= 0)
	                return true;
	            cmp(a.from, b.from) < 0
	                ? (a = (nextResult1 = i1.next(b.from)).value)
	                : (b = (nextResult2 = i2.next(a.from)).value);
	        }
	        return false;
	    }
	    function getRangeSetIterator(node) {
	        var state = isEmptyRange(node) ? null : { s: 0, n: node };
	        return {
	            next: function (key) {
	                var keyProvided = arguments.length > 0;
	                while (state) {
	                    switch (state.s) {
	                        case 0:
	                            state.s = 1;
	                            if (keyProvided) {
	                                while (state.n.l && cmp(key, state.n.from) < 0)
	                                    state = { up: state, n: state.n.l, s: 1 };
	                            }
	                            else {
	                                while (state.n.l)
	                                    state = { up: state, n: state.n.l, s: 1 };
	                            }
	                        case 1:
	                            state.s = 2;
	                            if (!keyProvided || cmp(key, state.n.to) <= 0)
	                                return { value: state.n, done: false };
	                        case 2:
	                            if (state.n.r) {
	                                state.s = 3;
	                                state = { up: state, n: state.n.r, s: 0 };
	                                continue;
	                            }
	                        case 3:
	                            state = state.up;
	                    }
	                }
	                return { done: true };
	            },
	        };
	    }
	    function rebalance(target) {
	        var _a, _b;
	        var diff = (((_a = target.r) === null || _a === void 0 ? void 0 : _a.d) || 0) - (((_b = target.l) === null || _b === void 0 ? void 0 : _b.d) || 0);
	        var r = diff > 1 ? "r" : diff < -1 ? "l" : "";
	        if (r) {
	            var l = r === "r" ? "l" : "r";
	            var rootClone = __assign({}, target);
	            var oldRootRight = target[r];
	            target.from = oldRootRight.from;
	            target.to = oldRootRight.to;
	            target[r] = oldRootRight[r];
	            rootClone[r] = oldRootRight[l];
	            target[l] = rootClone;
	            rootClone.d = computeDepth(rootClone);
	        }
	        target.d = computeDepth(target);
	    }
	    function computeDepth(_a) {
	        var r = _a.r, l = _a.l;
	        return (r ? (l ? Math.max(r.d, l.d) : r.d) : l ? l.d : 0) + 1;
	    }

	    function extendObservabilitySet(target, newSet) {
	        keys(newSet).forEach(function (part) {
	            if (target[part])
	                mergeRanges(target[part], newSet[part]);
	            else
	                target[part] = cloneSimpleObjectTree(newSet[part]);
	        });
	        return target;
	    }

	    function obsSetsOverlap(os1, os2) {
	        return os1.all || os2.all || Object.keys(os1).some(function (key) { return os2[key] && rangesOverlap(os2[key], os1[key]); });
	    }

	    var cache = {};

	    var unsignaledParts = {};
	    var isTaskEnqueued = false;
	    function signalSubscribersLazily(part, optimistic) {
	        extendObservabilitySet(unsignaledParts, part);
	        if (!isTaskEnqueued) {
	            isTaskEnqueued = true;
	            setTimeout(function () {
	                isTaskEnqueued = false;
	                var parts = unsignaledParts;
	                unsignaledParts = {};
	                signalSubscribersNow(parts, false);
	            }, 0);
	        }
	    }
	    function signalSubscribersNow(updatedParts, deleteAffectedCacheEntries) {
	        if (deleteAffectedCacheEntries === void 0) { deleteAffectedCacheEntries = false; }
	        var queriesToSignal = new Set();
	        if (updatedParts.all) {
	            for (var _i = 0, _a = Object.values(cache); _i < _a.length; _i++) {
	                var tblCache = _a[_i];
	                collectTableSubscribers(tblCache, updatedParts, queriesToSignal, deleteAffectedCacheEntries);
	            }
	        }
	        else {
	            for (var key in updatedParts) {
	                var parts = /^idb\:\/\/(.*)\/(.*)\//.exec(key);
	                if (parts) {
	                    var dbName = parts[1], tableName = parts[2];
	                    var tblCache = cache["idb://".concat(dbName, "/").concat(tableName)];
	                    if (tblCache)
	                        collectTableSubscribers(tblCache, updatedParts, queriesToSignal, deleteAffectedCacheEntries);
	                }
	            }
	        }
	        queriesToSignal.forEach(function (requery) { return requery(); });
	    }
	    function collectTableSubscribers(tblCache, updatedParts, outQueriesToSignal, deleteAffectedCacheEntries) {
	        var updatedEntryLists = [];
	        for (var _i = 0, _a = Object.entries(tblCache.queries.query); _i < _a.length; _i++) {
	            var _b = _a[_i], indexName = _b[0], entries = _b[1];
	            var filteredEntries = [];
	            for (var _c = 0, entries_1 = entries; _c < entries_1.length; _c++) {
	                var entry = entries_1[_c];
	                if (obsSetsOverlap(updatedParts, entry.obsSet)) {
	                    entry.subscribers.forEach(function (requery) { return outQueriesToSignal.add(requery); });
	                }
	                else if (deleteAffectedCacheEntries) {
	                    filteredEntries.push(entry);
	                }
	            }
	            if (deleteAffectedCacheEntries)
	                updatedEntryLists.push([indexName, filteredEntries]);
	        }
	        if (deleteAffectedCacheEntries) {
	            for (var _d = 0, updatedEntryLists_1 = updatedEntryLists; _d < updatedEntryLists_1.length; _d++) {
	                var _e = updatedEntryLists_1[_d], indexName = _e[0], filteredEntries = _e[1];
	                tblCache.queries.query[indexName] = filteredEntries;
	            }
	        }
	    }

	    function dexieOpen(db) {
	        var state = db._state;
	        var indexedDB = db._deps.indexedDB;
	        if (state.isBeingOpened || db.idbdb)
	            return state.dbReadyPromise.then(function () { return state.dbOpenError ?
	                rejection(state.dbOpenError) :
	                db; });
	        state.isBeingOpened = true;
	        state.dbOpenError = null;
	        state.openComplete = false;
	        var openCanceller = state.openCanceller;
	        var nativeVerToOpen = Math.round(db.verno * 10);
	        var schemaPatchMode = false;
	        function throwIfCancelled() {
	            if (state.openCanceller !== openCanceller)
	                throw new exceptions.DatabaseClosed('db.open() was cancelled');
	        }
	        var resolveDbReady = state.dbReadyResolve,
	        upgradeTransaction = null, wasCreated = false;
	        var tryOpenDB = function () { return new DexiePromise(function (resolve, reject) {
	            throwIfCancelled();
	            if (!indexedDB)
	                throw new exceptions.MissingAPI();
	            var dbName = db.name;
	            var req = state.autoSchema || !nativeVerToOpen ?
	                indexedDB.open(dbName) :
	                indexedDB.open(dbName, nativeVerToOpen);
	            if (!req)
	                throw new exceptions.MissingAPI();
	            req.onerror = eventRejectHandler(reject);
	            req.onblocked = wrap(db._fireOnBlocked);
	            req.onupgradeneeded = wrap(function (e) {
	                upgradeTransaction = req.transaction;
	                if (state.autoSchema && !db._options.allowEmptyDB) {
	                    req.onerror = preventDefault;
	                    upgradeTransaction.abort();
	                    req.result.close();
	                    var delreq = indexedDB.deleteDatabase(dbName);
	                    delreq.onsuccess = delreq.onerror = wrap(function () {
	                        reject(new exceptions.NoSuchDatabase("Database ".concat(dbName, " doesnt exist")));
	                    });
	                }
	                else {
	                    upgradeTransaction.onerror = eventRejectHandler(reject);
	                    var oldVer = e.oldVersion > Math.pow(2, 62) ? 0 : e.oldVersion;
	                    wasCreated = oldVer < 1;
	                    db.idbdb = req.result;
	                    if (schemaPatchMode) {
	                        patchCurrentVersion(db, upgradeTransaction);
	                    }
	                    runUpgraders(db, oldVer / 10, upgradeTransaction, reject);
	                }
	            }, reject);
	            req.onsuccess = wrap(function () {
	                upgradeTransaction = null;
	                var idbdb = db.idbdb = req.result;
	                var objectStoreNames = slice(idbdb.objectStoreNames);
	                if (objectStoreNames.length > 0)
	                    try {
	                        var tmpTrans = idbdb.transaction(safariMultiStoreFix(objectStoreNames), 'readonly');
	                        if (state.autoSchema)
	                            readGlobalSchema(db, idbdb, tmpTrans);
	                        else {
	                            adjustToExistingIndexNames(db, db._dbSchema, tmpTrans);
	                            if (!verifyInstalledSchema(db, tmpTrans) && !schemaPatchMode) {
	                                console.warn("Dexie SchemaDiff: Schema was extended without increasing the number passed to db.version(). Dexie will add missing parts and increment native version number to workaround this.");
	                                idbdb.close();
	                                nativeVerToOpen = idbdb.version + 1;
	                                schemaPatchMode = true;
	                                return resolve(tryOpenDB());
	                            }
	                        }
	                        generateMiddlewareStacks(db, tmpTrans);
	                    }
	                    catch (e) {
	                    }
	                connections.push(db);
	                idbdb.onversionchange = wrap(function (ev) {
	                    state.vcFired = true;
	                    db.on("versionchange").fire(ev);
	                });
	                idbdb.onclose = wrap(function (ev) {
	                    db.on("close").fire(ev);
	                });
	                if (wasCreated)
	                    _onDatabaseCreated(db._deps, dbName);
	                resolve();
	            }, reject);
	        }).catch(function (err) {
	            switch (err === null || err === void 0 ? void 0 : err.name) {
	                case "UnknownError":
	                    if (state.PR1398_maxLoop > 0) {
	                        state.PR1398_maxLoop--;
	                        console.warn('Dexie: Workaround for Chrome UnknownError on open()');
	                        return tryOpenDB();
	                    }
	                    break;
	                case "VersionError":
	                    if (nativeVerToOpen > 0) {
	                        nativeVerToOpen = 0;
	                        return tryOpenDB();
	                    }
	                    break;
	            }
	            return DexiePromise.reject(err);
	        }); };
	        return DexiePromise.race([
	            openCanceller,
	            (typeof navigator === 'undefined' ? DexiePromise.resolve() : idbReady()).then(tryOpenDB)
	        ]).then(function () {
	            throwIfCancelled();
	            state.onReadyBeingFired = [];
	            return DexiePromise.resolve(vip(function () { return db.on.ready.fire(db.vip); })).then(function fireRemainders() {
	                if (state.onReadyBeingFired.length > 0) {
	                    var remainders_1 = state.onReadyBeingFired.reduce(promisableChain, nop);
	                    state.onReadyBeingFired = [];
	                    return DexiePromise.resolve(vip(function () { return remainders_1(db.vip); })).then(fireRemainders);
	                }
	            });
	        }).finally(function () {
	            if (state.openCanceller === openCanceller) {
	                state.onReadyBeingFired = null;
	                state.isBeingOpened = false;
	            }
	        }).catch(function (err) {
	            state.dbOpenError = err;
	            try {
	                upgradeTransaction && upgradeTransaction.abort();
	            }
	            catch (_a) { }
	            if (openCanceller === state.openCanceller) {
	                db._close();
	            }
	            return rejection(err);
	        }).finally(function () {
	            state.openComplete = true;
	            resolveDbReady();
	        }).then(function () {
	            if (wasCreated) {
	                var everything_1 = {};
	                db.tables.forEach(function (table) {
	                    table.schema.indexes.forEach(function (idx) {
	                        if (idx.name)
	                            everything_1["idb://".concat(db.name, "/").concat(table.name, "/").concat(idx.name)] = new RangeSet(-Infinity, [[[]]]);
	                    });
	                    everything_1["idb://".concat(db.name, "/").concat(table.name, "/")] = everything_1["idb://".concat(db.name, "/").concat(table.name, "/:dels")] = new RangeSet(-Infinity, [[[]]]);
	                });
	                globalEvents(DEXIE_STORAGE_MUTATED_EVENT_NAME).fire(everything_1);
	                signalSubscribersNow(everything_1, true);
	            }
	            return db;
	        });
	    }

	    function awaitIterator(iterator) {
	        var callNext = function (result) { return iterator.next(result); }, doThrow = function (error) { return iterator.throw(error); }, onSuccess = step(callNext), onError = step(doThrow);
	        function step(getNext) {
	            return function (val) {
	                var next = getNext(val), value = next.value;
	                return next.done ? value :
	                    (!value || typeof value.then !== 'function' ?
	                        isArray(value) ? Promise.all(value).then(onSuccess, onError) : onSuccess(value) :
	                        value.then(onSuccess, onError));
	            };
	        }
	        return step(callNext)();
	    }

	    function extractTransactionArgs(mode, _tableArgs_, scopeFunc) {
	        var i = arguments.length;
	        if (i < 2)
	            throw new exceptions.InvalidArgument("Too few arguments");
	        var args = new Array(i - 1);
	        while (--i)
	            args[i - 1] = arguments[i];
	        scopeFunc = args.pop();
	        var tables = flatten(args);
	        return [mode, tables, scopeFunc];
	    }
	    function enterTransactionScope(db, mode, storeNames, parentTransaction, scopeFunc) {
	        return DexiePromise.resolve().then(function () {
	            var transless = PSD.transless || PSD;
	            var trans = db._createTransaction(mode, storeNames, db._dbSchema, parentTransaction);
	            trans.explicit = true;
	            var zoneProps = {
	                trans: trans,
	                transless: transless
	            };
	            if (parentTransaction) {
	                trans.idbtrans = parentTransaction.idbtrans;
	            }
	            else {
	                try {
	                    trans.create();
	                    trans.idbtrans._explicit = true;
	                    db._state.PR1398_maxLoop = 3;
	                }
	                catch (ex) {
	                    if (ex.name === errnames.InvalidState && db.isOpen() && --db._state.PR1398_maxLoop > 0) {
	                        console.warn('Dexie: Need to reopen db');
	                        db.close({ disableAutoOpen: false });
	                        return db.open().then(function () { return enterTransactionScope(db, mode, storeNames, null, scopeFunc); });
	                    }
	                    return rejection(ex);
	                }
	            }
	            var scopeFuncIsAsync = isAsyncFunction(scopeFunc);
	            if (scopeFuncIsAsync) {
	                incrementExpectedAwaits();
	            }
	            var returnValue;
	            var promiseFollowed = DexiePromise.follow(function () {
	                returnValue = scopeFunc.call(trans, trans);
	                if (returnValue) {
	                    if (scopeFuncIsAsync) {
	                        var decrementor = decrementExpectedAwaits.bind(null, null);
	                        returnValue.then(decrementor, decrementor);
	                    }
	                    else if (typeof returnValue.next === 'function' && typeof returnValue.throw === 'function') {
	                        returnValue = awaitIterator(returnValue);
	                    }
	                }
	            }, zoneProps);
	            return (returnValue && typeof returnValue.then === 'function' ?
	                DexiePromise.resolve(returnValue).then(function (x) { return trans.active ?
	                    x
	                    : rejection(new exceptions.PrematureCommit("Transaction committed too early. See http://bit.ly/2kdckMn")); })
	                : promiseFollowed.then(function () { return returnValue; })).then(function (x) {
	                if (parentTransaction)
	                    trans._resolve();
	                return trans._completion.then(function () { return x; });
	            }).catch(function (e) {
	                trans._reject(e);
	                return rejection(e);
	            });
	        });
	    }

	    function pad(a, value, count) {
	        var result = isArray(a) ? a.slice() : [a];
	        for (var i = 0; i < count; ++i)
	            result.push(value);
	        return result;
	    }
	    function createVirtualIndexMiddleware(down) {
	        return __assign(__assign({}, down), { table: function (tableName) {
	                var table = down.table(tableName);
	                var schema = table.schema;
	                var indexLookup = {};
	                var allVirtualIndexes = [];
	                function addVirtualIndexes(keyPath, keyTail, lowLevelIndex) {
	                    var keyPathAlias = getKeyPathAlias(keyPath);
	                    var indexList = (indexLookup[keyPathAlias] = indexLookup[keyPathAlias] || []);
	                    var keyLength = keyPath == null ? 0 : typeof keyPath === 'string' ? 1 : keyPath.length;
	                    var isVirtual = keyTail > 0;
	                    var virtualIndex = __assign(__assign({}, lowLevelIndex), { name: isVirtual
	                            ? "".concat(keyPathAlias, "(virtual-from:").concat(lowLevelIndex.name, ")")
	                            : lowLevelIndex.name, lowLevelIndex: lowLevelIndex, isVirtual: isVirtual, keyTail: keyTail, keyLength: keyLength, extractKey: getKeyExtractor(keyPath), unique: !isVirtual && lowLevelIndex.unique });
	                    indexList.push(virtualIndex);
	                    if (!virtualIndex.isPrimaryKey) {
	                        allVirtualIndexes.push(virtualIndex);
	                    }
	                    if (keyLength > 1) {
	                        var virtualKeyPath = keyLength === 2 ?
	                            keyPath[0] :
	                            keyPath.slice(0, keyLength - 1);
	                        addVirtualIndexes(virtualKeyPath, keyTail + 1, lowLevelIndex);
	                    }
	                    indexList.sort(function (a, b) { return a.keyTail - b.keyTail; });
	                    return virtualIndex;
	                }
	                var primaryKey = addVirtualIndexes(schema.primaryKey.keyPath, 0, schema.primaryKey);
	                indexLookup[":id"] = [primaryKey];
	                for (var _i = 0, _a = schema.indexes; _i < _a.length; _i++) {
	                    var index = _a[_i];
	                    addVirtualIndexes(index.keyPath, 0, index);
	                }
	                function findBestIndex(keyPath) {
	                    var result = indexLookup[getKeyPathAlias(keyPath)];
	                    return result && result[0];
	                }
	                function translateRange(range, keyTail) {
	                    return {
	                        type: range.type === 1  ?
	                            2  :
	                            range.type,
	                        lower: pad(range.lower, range.lowerOpen ? down.MAX_KEY : down.MIN_KEY, keyTail),
	                        lowerOpen: true,
	                        upper: pad(range.upper, range.upperOpen ? down.MIN_KEY : down.MAX_KEY, keyTail),
	                        upperOpen: true
	                    };
	                }
	                function translateRequest(req) {
	                    var index = req.query.index;
	                    return index.isVirtual ? __assign(__assign({}, req), { query: {
	                            index: index.lowLevelIndex,
	                            range: translateRange(req.query.range, index.keyTail)
	                        } }) : req;
	                }
	                var result = __assign(__assign({}, table), { schema: __assign(__assign({}, schema), { primaryKey: primaryKey, indexes: allVirtualIndexes, getIndexByKeyPath: findBestIndex }), count: function (req) {
	                        return table.count(translateRequest(req));
	                    }, query: function (req) {
	                        return table.query(translateRequest(req));
	                    }, openCursor: function (req) {
	                        var _a = req.query.index, keyTail = _a.keyTail, isVirtual = _a.isVirtual, keyLength = _a.keyLength;
	                        if (!isVirtual)
	                            return table.openCursor(req);
	                        function createVirtualCursor(cursor) {
	                            function _continue(key) {
	                                key != null ?
	                                    cursor.continue(pad(key, req.reverse ? down.MAX_KEY : down.MIN_KEY, keyTail)) :
	                                    req.unique ?
	                                        cursor.continue(cursor.key.slice(0, keyLength)
	                                            .concat(req.reverse
	                                            ? down.MIN_KEY
	                                            : down.MAX_KEY, keyTail)) :
	                                        cursor.continue();
	                            }
	                            var virtualCursor = Object.create(cursor, {
	                                continue: { value: _continue },
	                                continuePrimaryKey: {
	                                    value: function (key, primaryKey) {
	                                        cursor.continuePrimaryKey(pad(key, down.MAX_KEY, keyTail), primaryKey);
	                                    }
	                                },
	                                primaryKey: {
	                                    get: function () {
	                                        return cursor.primaryKey;
	                                    }
	                                },
	                                key: {
	                                    get: function () {
	                                        var key = cursor.key;
	                                        return keyLength === 1 ?
	                                            key[0] :
	                                            key.slice(0, keyLength);
	                                    }
	                                },
	                                value: {
	                                    get: function () {
	                                        return cursor.value;
	                                    }
	                                }
	                            });
	                            return virtualCursor;
	                        }
	                        return table.openCursor(translateRequest(req))
	                            .then(function (cursor) { return cursor && createVirtualCursor(cursor); });
	                    } });
	                return result;
	            } });
	    }
	    var virtualIndexMiddleware = {
	        stack: "dbcore",
	        name: "VirtualIndexMiddleware",
	        level: 1,
	        create: createVirtualIndexMiddleware
	    };

	    function getObjectDiff(a, b, rv, prfx) {
	        rv = rv || {};
	        prfx = prfx || '';
	        keys(a).forEach(function (prop) {
	            if (!hasOwn(b, prop)) {
	                rv[prfx + prop] = undefined;
	            }
	            else {
	                var ap = a[prop], bp = b[prop];
	                if (typeof ap === 'object' && typeof bp === 'object' && ap && bp) {
	                    var apTypeName = toStringTag(ap);
	                    var bpTypeName = toStringTag(bp);
	                    if (apTypeName !== bpTypeName) {
	                        rv[prfx + prop] = b[prop];
	                    }
	                    else if (apTypeName === 'Object') {
	                        getObjectDiff(ap, bp, rv, prfx + prop + '.');
	                    }
	                    else if (ap !== bp) {
	                        rv[prfx + prop] = b[prop];
	                    }
	                }
	                else if (ap !== bp)
	                    rv[prfx + prop] = b[prop];
	            }
	        });
	        keys(b).forEach(function (prop) {
	            if (!hasOwn(a, prop)) {
	                rv[prfx + prop] = b[prop];
	            }
	        });
	        return rv;
	    }

	    function getEffectiveKeys(primaryKey, req) {
	        if (req.type === 'delete')
	            return req.keys;
	        return req.keys || req.values.map(primaryKey.extractKey);
	    }

	    var hooksMiddleware = {
	        stack: "dbcore",
	        name: "HooksMiddleware",
	        level: 2,
	        create: function (downCore) { return (__assign(__assign({}, downCore), { table: function (tableName) {
	                var downTable = downCore.table(tableName);
	                var primaryKey = downTable.schema.primaryKey;
	                var tableMiddleware = __assign(__assign({}, downTable), { mutate: function (req) {
	                        var dxTrans = PSD.trans;
	                        var _a = dxTrans.table(tableName).hook, deleting = _a.deleting, creating = _a.creating, updating = _a.updating;
	                        switch (req.type) {
	                            case 'add':
	                                if (creating.fire === nop)
	                                    break;
	                                return dxTrans._promise('readwrite', function () { return addPutOrDelete(req); }, true);
	                            case 'put':
	                                if (creating.fire === nop && updating.fire === nop)
	                                    break;
	                                return dxTrans._promise('readwrite', function () { return addPutOrDelete(req); }, true);
	                            case 'delete':
	                                if (deleting.fire === nop)
	                                    break;
	                                return dxTrans._promise('readwrite', function () { return addPutOrDelete(req); }, true);
	                            case 'deleteRange':
	                                if (deleting.fire === nop)
	                                    break;
	                                return dxTrans._promise('readwrite', function () { return deleteRange(req); }, true);
	                        }
	                        return downTable.mutate(req);
	                        function addPutOrDelete(req) {
	                            var dxTrans = PSD.trans;
	                            var keys = req.keys || getEffectiveKeys(primaryKey, req);
	                            if (!keys)
	                                throw new Error("Keys missing");
	                            req = req.type === 'add' || req.type === 'put' ? __assign(__assign({}, req), { keys: keys }) : __assign({}, req);
	                            if (req.type !== 'delete')
	                                req.values = __spreadArray([], req.values, true);
	                            if (req.keys)
	                                req.keys = __spreadArray([], req.keys, true);
	                            return getExistingValues(downTable, req, keys).then(function (existingValues) {
	                                var contexts = keys.map(function (key, i) {
	                                    var existingValue = existingValues[i];
	                                    var ctx = { onerror: null, onsuccess: null };
	                                    if (req.type === 'delete') {
	                                        deleting.fire.call(ctx, key, existingValue, dxTrans);
	                                    }
	                                    else if (req.type === 'add' || existingValue === undefined) {
	                                        var generatedPrimaryKey = creating.fire.call(ctx, key, req.values[i], dxTrans);
	                                        if (key == null && generatedPrimaryKey != null) {
	                                            key = generatedPrimaryKey;
	                                            req.keys[i] = key;
	                                            if (!primaryKey.outbound) {
	                                                setByKeyPath(req.values[i], primaryKey.keyPath, key);
	                                            }
	                                        }
	                                    }
	                                    else {
	                                        var objectDiff = getObjectDiff(existingValue, req.values[i]);
	                                        var additionalChanges_1 = updating.fire.call(ctx, objectDiff, key, existingValue, dxTrans);
	                                        if (additionalChanges_1) {
	                                            var requestedValue_1 = req.values[i];
	                                            Object.keys(additionalChanges_1).forEach(function (keyPath) {
	                                                if (hasOwn(requestedValue_1, keyPath)) {
	                                                    requestedValue_1[keyPath] = additionalChanges_1[keyPath];
	                                                }
	                                                else {
	                                                    setByKeyPath(requestedValue_1, keyPath, additionalChanges_1[keyPath]);
	                                                }
	                                            });
	                                        }
	                                    }
	                                    return ctx;
	                                });
	                                return downTable.mutate(req).then(function (_a) {
	                                    var failures = _a.failures, results = _a.results, numFailures = _a.numFailures, lastResult = _a.lastResult;
	                                    for (var i = 0; i < keys.length; ++i) {
	                                        var primKey = results ? results[i] : keys[i];
	                                        var ctx = contexts[i];
	                                        if (primKey == null) {
	                                            ctx.onerror && ctx.onerror(failures[i]);
	                                        }
	                                        else {
	                                            ctx.onsuccess && ctx.onsuccess(req.type === 'put' && existingValues[i] ?
	                                                req.values[i] :
	                                                primKey
	                                            );
	                                        }
	                                    }
	                                    return { failures: failures, results: results, numFailures: numFailures, lastResult: lastResult };
	                                }).catch(function (error) {
	                                    contexts.forEach(function (ctx) { return ctx.onerror && ctx.onerror(error); });
	                                    return Promise.reject(error);
	                                });
	                            });
	                        }
	                        function deleteRange(req) {
	                            return deleteNextChunk(req.trans, req.range, 10000);
	                        }
	                        function deleteNextChunk(trans, range, limit) {
	                            return downTable.query({ trans: trans, values: false, query: { index: primaryKey, range: range }, limit: limit })
	                                .then(function (_a) {
	                                var result = _a.result;
	                                return addPutOrDelete({ type: 'delete', keys: result, trans: trans }).then(function (res) {
	                                    if (res.numFailures > 0)
	                                        return Promise.reject(res.failures[0]);
	                                    if (result.length < limit) {
	                                        return { failures: [], numFailures: 0, lastResult: undefined };
	                                    }
	                                    else {
	                                        return deleteNextChunk(trans, __assign(__assign({}, range), { lower: result[result.length - 1], lowerOpen: true }), limit);
	                                    }
	                                });
	                            });
	                        }
	                    } });
	                return tableMiddleware;
	            } })); }
	    };
	    function getExistingValues(table, req, effectiveKeys) {
	        return req.type === "add"
	            ? Promise.resolve([])
	            : table.getMany({ trans: req.trans, keys: effectiveKeys, cache: "immutable" });
	    }

	    function getFromTransactionCache(keys, cache, clone) {
	        try {
	            if (!cache)
	                return null;
	            if (cache.keys.length < keys.length)
	                return null;
	            var result = [];
	            for (var i = 0, j = 0; i < cache.keys.length && j < keys.length; ++i) {
	                if (cmp(cache.keys[i], keys[j]) !== 0)
	                    continue;
	                result.push(clone ? deepClone(cache.values[i]) : cache.values[i]);
	                ++j;
	            }
	            return result.length === keys.length ? result : null;
	        }
	        catch (_a) {
	            return null;
	        }
	    }
	    var cacheExistingValuesMiddleware = {
	        stack: "dbcore",
	        level: -1,
	        create: function (core) {
	            return {
	                table: function (tableName) {
	                    var table = core.table(tableName);
	                    return __assign(__assign({}, table), { getMany: function (req) {
	                            if (!req.cache) {
	                                return table.getMany(req);
	                            }
	                            var cachedResult = getFromTransactionCache(req.keys, req.trans["_cache"], req.cache === "clone");
	                            if (cachedResult) {
	                                return DexiePromise.resolve(cachedResult);
	                            }
	                            return table.getMany(req).then(function (res) {
	                                req.trans["_cache"] = {
	                                    keys: req.keys,
	                                    values: req.cache === "clone" ? deepClone(res) : res,
	                                };
	                                return res;
	                            });
	                        }, mutate: function (req) {
	                            if (req.type !== "add")
	                                req.trans["_cache"] = null;
	                            return table.mutate(req);
	                        } });
	                },
	            };
	        },
	    };

	    function isCachableContext(ctx, table) {
	        return (ctx.trans.mode === 'readonly' &&
	            !!ctx.subscr &&
	            !ctx.trans.explicit &&
	            ctx.trans.db._options.cache !== 'disabled' &&
	            !table.schema.primaryKey.outbound);
	    }

	    function isCachableRequest(type, req) {
	        switch (type) {
	            case 'query':
	                return req.values && !req.unique;
	            case 'get':
	                return false;
	            case 'getMany':
	                return false;
	            case 'count':
	                return false;
	            case 'openCursor':
	                return false;
	        }
	    }

	    var observabilityMiddleware = {
	        stack: "dbcore",
	        level: 0,
	        name: "Observability",
	        create: function (core) {
	            var dbName = core.schema.name;
	            var FULL_RANGE = new RangeSet(core.MIN_KEY, core.MAX_KEY);
	            return __assign(__assign({}, core), { transaction: function (stores, mode, options) {
	                    if (PSD.subscr && mode !== 'readonly') {
	                        throw new exceptions.ReadOnly("Readwrite transaction in liveQuery context. Querier source: ".concat(PSD.querier));
	                    }
	                    return core.transaction(stores, mode, options);
	                }, table: function (tableName) {
	                    var table = core.table(tableName);
	                    var schema = table.schema;
	                    var primaryKey = schema.primaryKey, indexes = schema.indexes;
	                    var extractKey = primaryKey.extractKey, outbound = primaryKey.outbound;
	                    var indexesWithAutoIncPK = primaryKey.autoIncrement && indexes.filter(function (index) { return index.compound && index.keyPath.includes(primaryKey.keyPath); });
	                    var tableClone = __assign(__assign({}, table), { mutate: function (req) {
	                            var trans = req.trans;
	                            var mutatedParts = req.mutatedParts || (req.mutatedParts = {});
	                            var getRangeSet = function (indexName) {
	                                var part = "idb://".concat(dbName, "/").concat(tableName, "/").concat(indexName);
	                                return (mutatedParts[part] ||
	                                    (mutatedParts[part] = new RangeSet()));
	                            };
	                            var pkRangeSet = getRangeSet("");
	                            var delsRangeSet = getRangeSet(":dels");
	                            var type = req.type;
	                            var _a = req.type === "deleteRange"
	                                ? [req.range]
	                                : req.type === "delete"
	                                    ? [req.keys]
	                                    : req.values.length < 50
	                                        ? [getEffectiveKeys(primaryKey, req).filter(function (id) { return id; }), req.values]
	                                        : [], keys = _a[0], newObjs = _a[1];
	                            var oldCache = req.trans["_cache"];
	                            if (isArray(keys)) {
	                                pkRangeSet.addKeys(keys);
	                                var oldObjs = type === 'delete' || keys.length === newObjs.length ? getFromTransactionCache(keys, oldCache) : null;
	                                if (!oldObjs) {
	                                    delsRangeSet.addKeys(keys);
	                                }
	                                if (oldObjs || newObjs) {
	                                    trackAffectedIndexes(getRangeSet, schema, oldObjs, newObjs);
	                                }
	                            }
	                            else if (keys) {
	                                var range = { from: keys.lower, to: keys.upper };
	                                delsRangeSet.add(range);
	                                pkRangeSet.add(range);
	                            }
	                            else {
	                                pkRangeSet.add(FULL_RANGE);
	                                delsRangeSet.add(FULL_RANGE);
	                                schema.indexes.forEach(function (idx) { return getRangeSet(idx.name).add(FULL_RANGE); });
	                            }
	                            return table.mutate(req).then(function (res) {
	                                if (keys && (req.type === 'add' || req.type === 'put')) {
	                                    pkRangeSet.addKeys(res.results);
	                                    if (indexesWithAutoIncPK) {
	                                        indexesWithAutoIncPK.forEach(function (idx) {
	                                            var idxVals = req.values.map(function (v) { return idx.extractKey(v); });
	                                            var pkPos = idx.keyPath.findIndex(function (prop) { return prop === primaryKey.keyPath; });
	                                            res.results.forEach(function (pk) { return idxVals[pkPos] = pk; });
	                                            getRangeSet(idx.name).addKeys(idxVals);
	                                        });
	                                    }
	                                }
	                                trans.mutatedParts = extendObservabilitySet(trans.mutatedParts || {}, mutatedParts);
	                                return res;
	                            });
	                        } });
	                    var getRange = function (_a) {
	                        var _b, _c;
	                        var _d = _a.query, index = _d.index, range = _d.range;
	                        return [
	                            index,
	                            new RangeSet((_b = range.lower) !== null && _b !== void 0 ? _b : core.MIN_KEY, (_c = range.upper) !== null && _c !== void 0 ? _c : core.MAX_KEY),
	                        ];
	                    };
	                    var readSubscribers = {
	                        get: function (req) { return [primaryKey, new RangeSet(req.key)]; },
	                        getMany: function (req) { return [primaryKey, new RangeSet().addKeys(req.keys)]; },
	                        count: getRange,
	                        query: getRange,
	                        openCursor: getRange,
	                    };
	                    keys(readSubscribers).forEach(function (method) {
	                        tableClone[method] = function (req) {
	                            var subscr = PSD.subscr;
	                            var isLiveQuery = !!subscr;
	                            var cachable = isCachableContext(PSD, table) && isCachableRequest(method, req);
	                            var obsSet = cachable
	                                ? req.obsSet = {}
	                                : subscr;
	                            if (isLiveQuery) {
	                                var getRangeSet = function (indexName) {
	                                    var part = "idb://".concat(dbName, "/").concat(tableName, "/").concat(indexName);
	                                    return (obsSet[part] ||
	                                        (obsSet[part] = new RangeSet()));
	                                };
	                                var pkRangeSet_1 = getRangeSet("");
	                                var delsRangeSet_1 = getRangeSet(":dels");
	                                var _a = readSubscribers[method](req), queriedIndex = _a[0], queriedRanges = _a[1];
	                                if (method === 'query' && queriedIndex.isPrimaryKey && !req.values) {
	                                    delsRangeSet_1.add(queriedRanges);
	                                }
	                                else {
	                                    getRangeSet(queriedIndex.name || "").add(queriedRanges);
	                                }
	                                if (!queriedIndex.isPrimaryKey) {
	                                    if (method === "count") {
	                                        delsRangeSet_1.add(FULL_RANGE);
	                                    }
	                                    else {
	                                        var keysPromise_1 = method === "query" &&
	                                            outbound &&
	                                            req.values &&
	                                            table.query(__assign(__assign({}, req), { values: false }));
	                                        return table[method].apply(this, arguments).then(function (res) {
	                                            if (method === "query") {
	                                                if (outbound && req.values) {
	                                                    return keysPromise_1.then(function (_a) {
	                                                        var resultingKeys = _a.result;
	                                                        pkRangeSet_1.addKeys(resultingKeys);
	                                                        return res;
	                                                    });
	                                                }
	                                                var pKeys = req.values
	                                                    ? res.result.map(extractKey)
	                                                    : res.result;
	                                                if (req.values) {
	                                                    pkRangeSet_1.addKeys(pKeys);
	                                                }
	                                                else {
	                                                    delsRangeSet_1.addKeys(pKeys);
	                                                }
	                                            }
	                                            else if (method === "openCursor") {
	                                                var cursor_1 = res;
	                                                var wantValues_1 = req.values;
	                                                return (cursor_1 &&
	                                                    Object.create(cursor_1, {
	                                                        key: {
	                                                            get: function () {
	                                                                delsRangeSet_1.addKey(cursor_1.primaryKey);
	                                                                return cursor_1.key;
	                                                            },
	                                                        },
	                                                        primaryKey: {
	                                                            get: function () {
	                                                                var pkey = cursor_1.primaryKey;
	                                                                delsRangeSet_1.addKey(pkey);
	                                                                return pkey;
	                                                            },
	                                                        },
	                                                        value: {
	                                                            get: function () {
	                                                                wantValues_1 && pkRangeSet_1.addKey(cursor_1.primaryKey);
	                                                                return cursor_1.value;
	                                                            },
	                                                        },
	                                                    }));
	                                            }
	                                            return res;
	                                        });
	                                    }
	                                }
	                            }
	                            return table[method].apply(this, arguments);
	                        };
	                    });
	                    return tableClone;
	                } });
	        },
	    };
	    function trackAffectedIndexes(getRangeSet, schema, oldObjs, newObjs) {
	        function addAffectedIndex(ix) {
	            var rangeSet = getRangeSet(ix.name || "");
	            function extractKey(obj) {
	                return obj != null ? ix.extractKey(obj) : null;
	            }
	            var addKeyOrKeys = function (key) { return ix.multiEntry && isArray(key)
	                ? key.forEach(function (key) { return rangeSet.addKey(key); })
	                : rangeSet.addKey(key); };
	            (oldObjs || newObjs).forEach(function (_, i) {
	                var oldKey = oldObjs && extractKey(oldObjs[i]);
	                var newKey = newObjs && extractKey(newObjs[i]);
	                if (cmp(oldKey, newKey) !== 0) {
	                    if (oldKey != null)
	                        addKeyOrKeys(oldKey);
	                    if (newKey != null)
	                        addKeyOrKeys(newKey);
	                }
	            });
	        }
	        schema.indexes.forEach(addAffectedIndex);
	    }

	    function adjustOptimisticFromFailures(tblCache, req, res) {
	        if (res.numFailures === 0)
	            return req;
	        if (req.type === 'deleteRange') {
	            return null;
	        }
	        var numBulkOps = req.keys
	            ? req.keys.length
	            : 'values' in req && req.values
	                ? req.values.length
	                : 1;
	        if (res.numFailures === numBulkOps) {
	            return null;
	        }
	        var clone = __assign({}, req);
	        if (isArray(clone.keys)) {
	            clone.keys = clone.keys.filter(function (_, i) { return !(i in res.failures); });
	        }
	        if ('values' in clone && isArray(clone.values)) {
	            clone.values = clone.values.filter(function (_, i) { return !(i in res.failures); });
	        }
	        return clone;
	    }

	    function isAboveLower(key, range) {
	        return range.lower === undefined
	            ? true
	            : range.lowerOpen
	                ? cmp(key, range.lower) > 0
	                : cmp(key, range.lower) >= 0;
	    }
	    function isBelowUpper(key, range) {
	        return range.upper === undefined
	            ? true
	            : range.upperOpen
	                ? cmp(key, range.upper) < 0
	                : cmp(key, range.upper) <= 0;
	    }
	    function isWithinRange(key, range) {
	        return isAboveLower(key, range) && isBelowUpper(key, range);
	    }

	    function applyOptimisticOps(result, req, ops, table, cacheEntry, immutable) {
	        if (!ops || ops.length === 0)
	            return result;
	        var index = req.query.index;
	        var multiEntry = index.multiEntry;
	        var queryRange = req.query.range;
	        var primaryKey = table.schema.primaryKey;
	        var extractPrimKey = primaryKey.extractKey;
	        var extractIndex = index.extractKey;
	        var extractLowLevelIndex = (index.lowLevelIndex || index).extractKey;
	        var finalResult = ops.reduce(function (result, op) {
	            var modifedResult = result;
	            var includedValues = op.type === 'add' || op.type === 'put'
	                ? op.values.filter(function (v) {
	                    var key = extractIndex(v);
	                    return multiEntry && isArray(key)
	                        ? key.some(function (k) { return isWithinRange(k, queryRange); })
	                        : isWithinRange(key, queryRange);
	                }).map(function (v) {
	                    v = deepClone(v);
	                    if (immutable)
	                        Object.freeze(v);
	                    return v;
	                })
	                : [];
	            switch (op.type) {
	                case 'add':
	                    modifedResult = result.concat(req.values
	                        ? includedValues
	                        : includedValues.map(function (v) { return extractPrimKey(v); }));
	                    break;
	                case 'put':
	                    var keySet_1 = new RangeSet().addKeys(op.values.map(function (v) { return extractPrimKey(v); }));
	                    modifedResult = result
	                        .filter(function (item) {
	                        var key = req.values ? extractPrimKey(item) : item;
	                        return !rangesOverlap(new RangeSet(key), keySet_1);
	                    })
	                        .concat(req.values
	                        ? includedValues
	                        : includedValues.map(function (v) { return extractPrimKey(v); }));
	                    break;
	                case 'delete':
	                    var keysToDelete_1 = new RangeSet().addKeys(op.keys);
	                    modifedResult = result.filter(function (item) {
	                        var key = req.values ? extractPrimKey(item) : item;
	                        return !rangesOverlap(new RangeSet(key), keysToDelete_1);
	                    });
	                    break;
	                case 'deleteRange':
	                    var range_1 = op.range;
	                    modifedResult = result.filter(function (item) { return !isWithinRange(extractPrimKey(item), range_1); });
	                    break;
	            }
	            return modifedResult;
	        }, result);
	        if (finalResult === result)
	            return result;
	        finalResult.sort(function (a, b) {
	            return cmp(extractLowLevelIndex(a), extractLowLevelIndex(b)) ||
	                cmp(extractPrimKey(a), extractPrimKey(b));
	        });
	        if (req.limit && req.limit < Infinity) {
	            if (finalResult.length > req.limit) {
	                finalResult.length = req.limit;
	            }
	            else if (result.length === req.limit && finalResult.length < req.limit) {
	                cacheEntry.dirty = true;
	            }
	        }
	        return immutable ? Object.freeze(finalResult) : finalResult;
	    }

	    function areRangesEqual(r1, r2) {
	        return (cmp(r1.lower, r2.lower) === 0 &&
	            cmp(r1.upper, r2.upper) === 0 &&
	            !!r1.lowerOpen === !!r2.lowerOpen &&
	            !!r1.upperOpen === !!r2.upperOpen);
	    }

	    function compareLowers(lower1, lower2, lowerOpen1, lowerOpen2) {
	        if (lower1 === undefined)
	            return lower2 !== undefined ? -1 : 0;
	        if (lower2 === undefined)
	            return 1;
	        var c = cmp(lower1, lower2);
	        if (c === 0) {
	            if (lowerOpen1 && lowerOpen2)
	                return 0;
	            if (lowerOpen1)
	                return 1;
	            if (lowerOpen2)
	                return -1;
	        }
	        return c;
	    }
	    function compareUppers(upper1, upper2, upperOpen1, upperOpen2) {
	        if (upper1 === undefined)
	            return upper2 !== undefined ? 1 : 0;
	        if (upper2 === undefined)
	            return -1;
	        var c = cmp(upper1, upper2);
	        if (c === 0) {
	            if (upperOpen1 && upperOpen2)
	                return 0;
	            if (upperOpen1)
	                return -1;
	            if (upperOpen2)
	                return 1;
	        }
	        return c;
	    }
	    function isSuperRange(r1, r2) {
	        return (compareLowers(r1.lower, r2.lower, r1.lowerOpen, r2.lowerOpen) <= 0 &&
	            compareUppers(r1.upper, r2.upper, r1.upperOpen, r2.upperOpen) >= 0);
	    }

	    function findCompatibleQuery(dbName, tableName, type, req) {
	        var tblCache = cache["idb://".concat(dbName, "/").concat(tableName)];
	        if (!tblCache)
	            return [];
	        var queries = tblCache.queries[type];
	        if (!queries)
	            return [null, false, tblCache, null];
	        var indexName = req.query ? req.query.index.name : null;
	        var entries = queries[indexName || ''];
	        if (!entries)
	            return [null, false, tblCache, null];
	        switch (type) {
	            case 'query':
	                var equalEntry = entries.find(function (entry) {
	                    return entry.req.limit === req.limit &&
	                        entry.req.values === req.values &&
	                        areRangesEqual(entry.req.query.range, req.query.range);
	                });
	                if (equalEntry)
	                    return [
	                        equalEntry,
	                        true,
	                        tblCache,
	                        entries,
	                    ];
	                var superEntry = entries.find(function (entry) {
	                    var limit = 'limit' in entry.req ? entry.req.limit : Infinity;
	                    return (limit >= req.limit &&
	                        (req.values ? entry.req.values : true) &&
	                        isSuperRange(entry.req.query.range, req.query.range));
	                });
	                return [superEntry, false, tblCache, entries];
	            case 'count':
	                var countQuery = entries.find(function (entry) {
	                    return areRangesEqual(entry.req.query.range, req.query.range);
	                });
	                return [countQuery, !!countQuery, tblCache, entries];
	        }
	    }

	    function subscribeToCacheEntry(cacheEntry, container, requery, signal) {
	        cacheEntry.subscribers.add(requery);
	        signal.addEventListener("abort", function () {
	            cacheEntry.subscribers.delete(requery);
	            if (cacheEntry.subscribers.size === 0) {
	                enqueForDeletion(cacheEntry, container);
	            }
	        });
	    }
	    function enqueForDeletion(cacheEntry, container) {
	        setTimeout(function () {
	            if (cacheEntry.subscribers.size === 0) {
	                delArrayItem(container, cacheEntry);
	            }
	        }, 3000);
	    }

	    var cacheMiddleware = {
	        stack: 'dbcore',
	        level: 0,
	        name: 'Cache',
	        create: function (core) {
	            var dbName = core.schema.name;
	            var coreMW = __assign(__assign({}, core), { transaction: function (stores, mode, options) {
	                    var idbtrans = core.transaction(stores, mode, options);
	                    if (mode === 'readwrite') {
	                        var ac_1 = new AbortController();
	                        var signal = ac_1.signal;
	                        var endTransaction = function (wasCommitted) { return function () {
	                            ac_1.abort();
	                            if (mode === 'readwrite') {
	                                var affectedSubscribers_1 = new Set();
	                                for (var _i = 0, stores_1 = stores; _i < stores_1.length; _i++) {
	                                    var storeName = stores_1[_i];
	                                    var tblCache = cache["idb://".concat(dbName, "/").concat(storeName)];
	                                    if (tblCache) {
	                                        var table = core.table(storeName);
	                                        var ops = tblCache.optimisticOps.filter(function (op) { return op.trans === idbtrans; });
	                                        if (idbtrans._explicit && wasCommitted && idbtrans.mutatedParts) {
	                                            for (var _a = 0, _b = Object.values(tblCache.queries.query); _a < _b.length; _a++) {
	                                                var entries = _b[_a];
	                                                for (var _c = 0, _d = entries.slice(); _c < _d.length; _c++) {
	                                                    var entry = _d[_c];
	                                                    if (obsSetsOverlap(entry.obsSet, idbtrans.mutatedParts)) {
	                                                        delArrayItem(entries, entry);
	                                                        entry.subscribers.forEach(function (requery) { return affectedSubscribers_1.add(requery); });
	                                                    }
	                                                }
	                                            }
	                                        }
	                                        else if (ops.length > 0) {
	                                            tblCache.optimisticOps = tblCache.optimisticOps.filter(function (op) { return op.trans !== idbtrans; });
	                                            for (var _e = 0, _f = Object.values(tblCache.queries.query); _e < _f.length; _e++) {
	                                                var entries = _f[_e];
	                                                for (var _g = 0, _h = entries.slice(); _g < _h.length; _g++) {
	                                                    var entry = _h[_g];
	                                                    if (entry.res != null &&
	                                                        idbtrans.mutatedParts
	    ) {
	                                                        if (wasCommitted && !entry.dirty) {
	                                                            var freezeResults = Object.isFrozen(entry.res);
	                                                            var modRes = applyOptimisticOps(entry.res, entry.req, ops, table, entry, freezeResults);
	                                                            if (entry.dirty) {
	                                                                delArrayItem(entries, entry);
	                                                                entry.subscribers.forEach(function (requery) { return affectedSubscribers_1.add(requery); });
	                                                            }
	                                                            else if (modRes !== entry.res) {
	                                                                entry.res = modRes;
	                                                                entry.promise = DexiePromise.resolve({ result: modRes });
	                                                            }
	                                                        }
	                                                        else {
	                                                            if (entry.dirty) {
	                                                                delArrayItem(entries, entry);
	                                                            }
	                                                            entry.subscribers.forEach(function (requery) { return affectedSubscribers_1.add(requery); });
	                                                        }
	                                                    }
	                                                }
	                                            }
	                                        }
	                                    }
	                                }
	                                affectedSubscribers_1.forEach(function (requery) { return requery(); });
	                            }
	                        }; };
	                        idbtrans.addEventListener('abort', endTransaction(false), {
	                            signal: signal,
	                        });
	                        idbtrans.addEventListener('error', endTransaction(false), {
	                            signal: signal,
	                        });
	                        idbtrans.addEventListener('complete', endTransaction(true), {
	                            signal: signal,
	                        });
	                    }
	                    return idbtrans;
	                }, table: function (tableName) {
	                    var downTable = core.table(tableName);
	                    var primKey = downTable.schema.primaryKey;
	                    var tableMW = __assign(__assign({}, downTable), { mutate: function (req) {
	                            var trans = PSD.trans;
	                            if (primKey.outbound ||
	                                trans.db._options.cache === 'disabled' ||
	                                trans.explicit
	                            ) {
	                                return downTable.mutate(req);
	                            }
	                            var tblCache = cache["idb://".concat(dbName, "/").concat(tableName)];
	                            if (!tblCache)
	                                return downTable.mutate(req);
	                            var promise = downTable.mutate(req);
	                            if ((req.type === 'add' || req.type === 'put') && (req.values.length >= 50 || getEffectiveKeys(primKey, req).some(function (key) { return key == null; }))) {
	                                promise.then(function (res) {
	                                    var reqWithResolvedKeys = __assign(__assign({}, req), { values: req.values.map(function (value, i) {
	                                            var _a;
	                                            var valueWithKey = ((_a = primKey.keyPath) === null || _a === void 0 ? void 0 : _a.includes('.'))
	                                                ? deepClone(value)
	                                                : __assign({}, value);
	                                            setByKeyPath(valueWithKey, primKey.keyPath, res.results[i]);
	                                            return valueWithKey;
	                                        }) });
	                                    var adjustedReq = adjustOptimisticFromFailures(tblCache, reqWithResolvedKeys, res);
	                                    tblCache.optimisticOps.push(adjustedReq);
	                                    queueMicrotask(function () { return req.mutatedParts && signalSubscribersLazily(req.mutatedParts); });
	                                });
	                            }
	                            else {
	                                tblCache.optimisticOps.push(req);
	                                req.mutatedParts && signalSubscribersLazily(req.mutatedParts);
	                                promise.then(function (res) {
	                                    if (res.numFailures > 0) {
	                                        delArrayItem(tblCache.optimisticOps, req);
	                                        var adjustedReq = adjustOptimisticFromFailures(tblCache, req, res);
	                                        if (adjustedReq) {
	                                            tblCache.optimisticOps.push(adjustedReq);
	                                        }
	                                        req.mutatedParts && signalSubscribersLazily(req.mutatedParts);
	                                    }
	                                });
	                                promise.catch(function () {
	                                    delArrayItem(tblCache.optimisticOps, req);
	                                    req.mutatedParts && signalSubscribersLazily(req.mutatedParts);
	                                });
	                            }
	                            return promise;
	                        }, query: function (req) {
	                            var _a;
	                            if (!isCachableContext(PSD, downTable) || !isCachableRequest("query", req))
	                                return downTable.query(req);
	                            var freezeResults = ((_a = PSD.trans) === null || _a === void 0 ? void 0 : _a.db._options.cache) === 'immutable';
	                            var _b = PSD, requery = _b.requery, signal = _b.signal;
	                            var _c = findCompatibleQuery(dbName, tableName, 'query', req), cacheEntry = _c[0], exactMatch = _c[1], tblCache = _c[2], container = _c[3];
	                            if (cacheEntry && exactMatch) {
	                                cacheEntry.obsSet = req.obsSet;
	                            }
	                            else {
	                                var promise = downTable.query(req).then(function (res) {
	                                    var result = res.result;
	                                    if (cacheEntry)
	                                        cacheEntry.res = result;
	                                    if (freezeResults) {
	                                        for (var i = 0, l = result.length; i < l; ++i) {
	                                            Object.freeze(result[i]);
	                                        }
	                                        Object.freeze(result);
	                                    }
	                                    else {
	                                        res.result = deepClone(result);
	                                    }
	                                    return res;
	                                }).catch(function (error) {
	                                    if (container && cacheEntry)
	                                        delArrayItem(container, cacheEntry);
	                                    return Promise.reject(error);
	                                });
	                                cacheEntry = {
	                                    obsSet: req.obsSet,
	                                    promise: promise,
	                                    subscribers: new Set(),
	                                    type: 'query',
	                                    req: req,
	                                    dirty: false,
	                                };
	                                if (container) {
	                                    container.push(cacheEntry);
	                                }
	                                else {
	                                    container = [cacheEntry];
	                                    if (!tblCache) {
	                                        tblCache = cache["idb://".concat(dbName, "/").concat(tableName)] = {
	                                            queries: {
	                                                query: {},
	                                                count: {},
	                                            },
	                                            objs: new Map(),
	                                            optimisticOps: [],
	                                            unsignaledParts: {}
	                                        };
	                                    }
	                                    tblCache.queries.query[req.query.index.name || ''] = container;
	                                }
	                            }
	                            subscribeToCacheEntry(cacheEntry, container, requery, signal);
	                            return cacheEntry.promise.then(function (res) {
	                                return {
	                                    result: applyOptimisticOps(res.result, req, tblCache === null || tblCache === void 0 ? void 0 : tblCache.optimisticOps, downTable, cacheEntry, freezeResults),
	                                };
	                            });
	                        } });
	                    return tableMW;
	                } });
	            return coreMW;
	        },
	    };

	    function vipify(target, vipDb) {
	        return new Proxy(target, {
	            get: function (target, prop, receiver) {
	                if (prop === 'db')
	                    return vipDb;
	                return Reflect.get(target, prop, receiver);
	            }
	        });
	    }

	    var Dexie$1 =  (function () {
	        function Dexie(name, options) {
	            var _this = this;
	            this._middlewares = {};
	            this.verno = 0;
	            var deps = Dexie.dependencies;
	            this._options = options = __assign({
	                addons: Dexie.addons, autoOpen: true,
	                indexedDB: deps.indexedDB, IDBKeyRange: deps.IDBKeyRange, cache: 'cloned' }, options);
	            this._deps = {
	                indexedDB: options.indexedDB,
	                IDBKeyRange: options.IDBKeyRange
	            };
	            var addons = options.addons;
	            this._dbSchema = {};
	            this._versions = [];
	            this._storeNames = [];
	            this._allTables = {};
	            this.idbdb = null;
	            this._novip = this;
	            var state = {
	                dbOpenError: null,
	                isBeingOpened: false,
	                onReadyBeingFired: null,
	                openComplete: false,
	                dbReadyResolve: nop,
	                dbReadyPromise: null,
	                cancelOpen: nop,
	                openCanceller: null,
	                autoSchema: true,
	                PR1398_maxLoop: 3,
	                autoOpen: options.autoOpen,
	            };
	            state.dbReadyPromise = new DexiePromise(function (resolve) {
	                state.dbReadyResolve = resolve;
	            });
	            state.openCanceller = new DexiePromise(function (_, reject) {
	                state.cancelOpen = reject;
	            });
	            this._state = state;
	            this.name = name;
	            this.on = Events(this, "populate", "blocked", "versionchange", "close", { ready: [promisableChain, nop] });
	            this.on.ready.subscribe = override(this.on.ready.subscribe, function (subscribe) {
	                return function (subscriber, bSticky) {
	                    Dexie.vip(function () {
	                        var state = _this._state;
	                        if (state.openComplete) {
	                            if (!state.dbOpenError)
	                                DexiePromise.resolve().then(subscriber);
	                            if (bSticky)
	                                subscribe(subscriber);
	                        }
	                        else if (state.onReadyBeingFired) {
	                            state.onReadyBeingFired.push(subscriber);
	                            if (bSticky)
	                                subscribe(subscriber);
	                        }
	                        else {
	                            subscribe(subscriber);
	                            var db_1 = _this;
	                            if (!bSticky)
	                                subscribe(function unsubscribe() {
	                                    db_1.on.ready.unsubscribe(subscriber);
	                                    db_1.on.ready.unsubscribe(unsubscribe);
	                                });
	                        }
	                    });
	                };
	            });
	            this.Collection = createCollectionConstructor(this);
	            this.Table = createTableConstructor(this);
	            this.Transaction = createTransactionConstructor(this);
	            this.Version = createVersionConstructor(this);
	            this.WhereClause = createWhereClauseConstructor(this);
	            this.on("versionchange", function (ev) {
	                if (ev.newVersion > 0)
	                    console.warn("Another connection wants to upgrade database '".concat(_this.name, "'. Closing db now to resume the upgrade."));
	                else
	                    console.warn("Another connection wants to delete database '".concat(_this.name, "'. Closing db now to resume the delete request."));
	                _this.close({ disableAutoOpen: false });
	            });
	            this.on("blocked", function (ev) {
	                if (!ev.newVersion || ev.newVersion < ev.oldVersion)
	                    console.warn("Dexie.delete('".concat(_this.name, "') was blocked"));
	                else
	                    console.warn("Upgrade '".concat(_this.name, "' blocked by other connection holding version ").concat(ev.oldVersion / 10));
	            });
	            this._maxKey = getMaxKey(options.IDBKeyRange);
	            this._createTransaction = function (mode, storeNames, dbschema, parentTransaction) { return new _this.Transaction(mode, storeNames, dbschema, _this._options.chromeTransactionDurability, parentTransaction); };
	            this._fireOnBlocked = function (ev) {
	                _this.on("blocked").fire(ev);
	                connections
	                    .filter(function (c) { return c.name === _this.name && c !== _this && !c._state.vcFired; })
	                    .map(function (c) { return c.on("versionchange").fire(ev); });
	            };
	            this.use(cacheExistingValuesMiddleware);
	            this.use(cacheMiddleware);
	            this.use(observabilityMiddleware);
	            this.use(virtualIndexMiddleware);
	            this.use(hooksMiddleware);
	            var vipDB = new Proxy(this, {
	                get: function (_, prop, receiver) {
	                    if (prop === '_vip')
	                        return true;
	                    if (prop === 'table')
	                        return function (tableName) { return vipify(_this.table(tableName), vipDB); };
	                    var rv = Reflect.get(_, prop, receiver);
	                    if (rv instanceof Table)
	                        return vipify(rv, vipDB);
	                    if (prop === 'tables')
	                        return rv.map(function (t) { return vipify(t, vipDB); });
	                    if (prop === '_createTransaction')
	                        return function () {
	                            var tx = rv.apply(this, arguments);
	                            return vipify(tx, vipDB);
	                        };
	                    return rv;
	                }
	            });
	            this.vip = vipDB;
	            addons.forEach(function (addon) { return addon(_this); });
	        }
	        Dexie.prototype.version = function (versionNumber) {
	            if (isNaN(versionNumber) || versionNumber < 0.1)
	                throw new exceptions.Type("Given version is not a positive number");
	            versionNumber = Math.round(versionNumber * 10) / 10;
	            if (this.idbdb || this._state.isBeingOpened)
	                throw new exceptions.Schema("Cannot add version when database is open");
	            this.verno = Math.max(this.verno, versionNumber);
	            var versions = this._versions;
	            var versionInstance = versions.filter(function (v) { return v._cfg.version === versionNumber; })[0];
	            if (versionInstance)
	                return versionInstance;
	            versionInstance = new this.Version(versionNumber);
	            versions.push(versionInstance);
	            versions.sort(lowerVersionFirst);
	            versionInstance.stores({});
	            this._state.autoSchema = false;
	            return versionInstance;
	        };
	        Dexie.prototype._whenReady = function (fn) {
	            var _this = this;
	            return (this.idbdb && (this._state.openComplete || PSD.letThrough || this._vip)) ? fn() : new DexiePromise(function (resolve, reject) {
	                if (_this._state.openComplete) {
	                    return reject(new exceptions.DatabaseClosed(_this._state.dbOpenError));
	                }
	                if (!_this._state.isBeingOpened) {
	                    if (!_this._state.autoOpen) {
	                        reject(new exceptions.DatabaseClosed());
	                        return;
	                    }
	                    _this.open().catch(nop);
	                }
	                _this._state.dbReadyPromise.then(resolve, reject);
	            }).then(fn);
	        };
	        Dexie.prototype.use = function (_a) {
	            var stack = _a.stack, create = _a.create, level = _a.level, name = _a.name;
	            if (name)
	                this.unuse({ stack: stack, name: name });
	            var middlewares = this._middlewares[stack] || (this._middlewares[stack] = []);
	            middlewares.push({ stack: stack, create: create, level: level == null ? 10 : level, name: name });
	            middlewares.sort(function (a, b) { return a.level - b.level; });
	            return this;
	        };
	        Dexie.prototype.unuse = function (_a) {
	            var stack = _a.stack, name = _a.name, create = _a.create;
	            if (stack && this._middlewares[stack]) {
	                this._middlewares[stack] = this._middlewares[stack].filter(function (mw) {
	                    return create ? mw.create !== create :
	                        name ? mw.name !== name :
	                            false;
	                });
	            }
	            return this;
	        };
	        Dexie.prototype.open = function () {
	            var _this = this;
	            return usePSD(globalPSD,
	            function () { return dexieOpen(_this); });
	        };
	        Dexie.prototype._close = function () {
	            var state = this._state;
	            var idx = connections.indexOf(this);
	            if (idx >= 0)
	                connections.splice(idx, 1);
	            if (this.idbdb) {
	                try {
	                    this.idbdb.close();
	                }
	                catch (e) { }
	                this.idbdb = null;
	            }
	            if (!state.isBeingOpened) {
	                state.dbReadyPromise = new DexiePromise(function (resolve) {
	                    state.dbReadyResolve = resolve;
	                });
	                state.openCanceller = new DexiePromise(function (_, reject) {
	                    state.cancelOpen = reject;
	                });
	            }
	        };
	        Dexie.prototype.close = function (_a) {
	            var _b = _a === void 0 ? { disableAutoOpen: true } : _a, disableAutoOpen = _b.disableAutoOpen;
	            var state = this._state;
	            if (disableAutoOpen) {
	                if (state.isBeingOpened) {
	                    state.cancelOpen(new exceptions.DatabaseClosed());
	                }
	                this._close();
	                state.autoOpen = false;
	                state.dbOpenError = new exceptions.DatabaseClosed();
	            }
	            else {
	                this._close();
	                state.autoOpen = this._options.autoOpen ||
	                    state.isBeingOpened;
	                state.openComplete = false;
	                state.dbOpenError = null;
	            }
	        };
	        Dexie.prototype.delete = function (closeOptions) {
	            var _this = this;
	            if (closeOptions === void 0) { closeOptions = { disableAutoOpen: true }; }
	            var hasInvalidArguments = arguments.length > 0 && typeof arguments[0] !== 'object';
	            var state = this._state;
	            return new DexiePromise(function (resolve, reject) {
	                var doDelete = function () {
	                    _this.close(closeOptions);
	                    var req = _this._deps.indexedDB.deleteDatabase(_this.name);
	                    req.onsuccess = wrap(function () {
	                        _onDatabaseDeleted(_this._deps, _this.name);
	                        resolve();
	                    });
	                    req.onerror = eventRejectHandler(reject);
	                    req.onblocked = _this._fireOnBlocked;
	                };
	                if (hasInvalidArguments)
	                    throw new exceptions.InvalidArgument("Invalid closeOptions argument to db.delete()");
	                if (state.isBeingOpened) {
	                    state.dbReadyPromise.then(doDelete);
	                }
	                else {
	                    doDelete();
	                }
	            });
	        };
	        Dexie.prototype.backendDB = function () {
	            return this.idbdb;
	        };
	        Dexie.prototype.isOpen = function () {
	            return this.idbdb !== null;
	        };
	        Dexie.prototype.hasBeenClosed = function () {
	            var dbOpenError = this._state.dbOpenError;
	            return dbOpenError && (dbOpenError.name === 'DatabaseClosed');
	        };
	        Dexie.prototype.hasFailed = function () {
	            return this._state.dbOpenError !== null;
	        };
	        Dexie.prototype.dynamicallyOpened = function () {
	            return this._state.autoSchema;
	        };
	        Object.defineProperty(Dexie.prototype, "tables", {
	            get: function () {
	                var _this = this;
	                return keys(this._allTables).map(function (name) { return _this._allTables[name]; });
	            },
	            enumerable: false,
	            configurable: true
	        });
	        Dexie.prototype.transaction = function () {
	            var args = extractTransactionArgs.apply(this, arguments);
	            return this._transaction.apply(this, args);
	        };
	        Dexie.prototype._transaction = function (mode, tables, scopeFunc) {
	            var _this = this;
	            var parentTransaction = PSD.trans;
	            if (!parentTransaction || parentTransaction.db !== this || mode.indexOf('!') !== -1)
	                parentTransaction = null;
	            var onlyIfCompatible = mode.indexOf('?') !== -1;
	            mode = mode.replace('!', '').replace('?', '');
	            var idbMode, storeNames;
	            try {
	                storeNames = tables.map(function (table) {
	                    var storeName = table instanceof _this.Table ? table.name : table;
	                    if (typeof storeName !== 'string')
	                        throw new TypeError("Invalid table argument to Dexie.transaction(). Only Table or String are allowed");
	                    return storeName;
	                });
	                if (mode == "r" || mode === READONLY)
	                    idbMode = READONLY;
	                else if (mode == "rw" || mode == READWRITE)
	                    idbMode = READWRITE;
	                else
	                    throw new exceptions.InvalidArgument("Invalid transaction mode: " + mode);
	                if (parentTransaction) {
	                    if (parentTransaction.mode === READONLY && idbMode === READWRITE) {
	                        if (onlyIfCompatible) {
	                            parentTransaction = null;
	                        }
	                        else
	                            throw new exceptions.SubTransaction("Cannot enter a sub-transaction with READWRITE mode when parent transaction is READONLY");
	                    }
	                    if (parentTransaction) {
	                        storeNames.forEach(function (storeName) {
	                            if (parentTransaction && parentTransaction.storeNames.indexOf(storeName) === -1) {
	                                if (onlyIfCompatible) {
	                                    parentTransaction = null;
	                                }
	                                else
	                                    throw new exceptions.SubTransaction("Table " + storeName +
	                                        " not included in parent transaction.");
	                            }
	                        });
	                    }
	                    if (onlyIfCompatible && parentTransaction && !parentTransaction.active) {
	                        parentTransaction = null;
	                    }
	                }
	            }
	            catch (e) {
	                return parentTransaction ?
	                    parentTransaction._promise(null, function (_, reject) { reject(e); }) :
	                    rejection(e);
	            }
	            var enterTransaction = enterTransactionScope.bind(null, this, idbMode, storeNames, parentTransaction, scopeFunc);
	            return (parentTransaction ?
	                parentTransaction._promise(idbMode, enterTransaction, "lock") :
	                PSD.trans ?
	                    usePSD(PSD.transless, function () { return _this._whenReady(enterTransaction); }) :
	                    this._whenReady(enterTransaction));
	        };
	        Dexie.prototype.table = function (tableName) {
	            if (!hasOwn(this._allTables, tableName)) {
	                throw new exceptions.InvalidTable("Table ".concat(tableName, " does not exist"));
	            }
	            return this._allTables[tableName];
	        };
	        return Dexie;
	    }());

	    var symbolObservable = typeof Symbol !== "undefined" && "observable" in Symbol
	        ? Symbol.observable
	        : "@@observable";
	    var Observable =  (function () {
	        function Observable(subscribe) {
	            this._subscribe = subscribe;
	        }
	        Observable.prototype.subscribe = function (x, error, complete) {
	            return this._subscribe(!x || typeof x === "function" ? { next: x, error: error, complete: complete } : x);
	        };
	        Observable.prototype[symbolObservable] = function () {
	            return this;
	        };
	        return Observable;
	    }());

	    var domDeps;
	    try {
	        domDeps = {
	            indexedDB: _global.indexedDB || _global.mozIndexedDB || _global.webkitIndexedDB || _global.msIndexedDB,
	            IDBKeyRange: _global.IDBKeyRange || _global.webkitIDBKeyRange
	        };
	    }
	    catch (e) {
	        domDeps = { indexedDB: null, IDBKeyRange: null };
	    }

	    function liveQuery(querier) {
	        var hasValue = false;
	        var currentValue;
	        var observable = new Observable(function (observer) {
	            var scopeFuncIsAsync = isAsyncFunction(querier);
	            function execute(ctx) {
	                var wasRootExec = beginMicroTickScope();
	                try {
	                    if (scopeFuncIsAsync) {
	                        incrementExpectedAwaits();
	                    }
	                    var rv = newScope(querier, ctx);
	                    if (scopeFuncIsAsync) {
	                        rv = rv.finally(decrementExpectedAwaits);
	                    }
	                    return rv;
	                }
	                finally {
	                    wasRootExec && endMicroTickScope();
	                }
	            }
	            var closed = false;
	            var abortController;
	            var accumMuts = {};
	            var currentObs = {};
	            var subscription = {
	                get closed() {
	                    return closed;
	                },
	                unsubscribe: function () {
	                    if (closed)
	                        return;
	                    closed = true;
	                    if (abortController)
	                        abortController.abort();
	                    if (startedListening)
	                        globalEvents.storagemutated.unsubscribe(mutationListener);
	                },
	            };
	            observer.start && observer.start(subscription);
	            var startedListening = false;
	            var doQuery = function () { return execInGlobalContext(_doQuery); };
	            function shouldNotify() {
	                return obsSetsOverlap(currentObs, accumMuts);
	            }
	            var mutationListener = function (parts) {
	                extendObservabilitySet(accumMuts, parts);
	                if (shouldNotify()) {
	                    doQuery();
	                }
	            };
	            var _doQuery = function () {
	                if (closed ||
	                    !domDeps.indexedDB)
	                 {
	                    return;
	                }
	                accumMuts = {};
	                var subscr = {};
	                if (abortController)
	                    abortController.abort();
	                abortController = new AbortController();
	                var ctx = {
	                    subscr: subscr,
	                    signal: abortController.signal,
	                    requery: doQuery,
	                    querier: querier,
	                    trans: null
	                };
	                var ret = execute(ctx);
	                Promise.resolve(ret).then(function (result) {
	                    hasValue = true;
	                    currentValue = result;
	                    if (closed || ctx.signal.aborted) {
	                        return;
	                    }
	                    accumMuts = {};
	                    currentObs = subscr;
	                    if (!objectIsEmpty(currentObs) && !startedListening) {
	                        globalEvents(DEXIE_STORAGE_MUTATED_EVENT_NAME, mutationListener);
	                        startedListening = true;
	                    }
	                    execInGlobalContext(function () { return !closed && observer.next && observer.next(result); });
	                }, function (err) {
	                    hasValue = false;
	                    if (!['DatabaseClosedError', 'AbortError'].includes(err === null || err === void 0 ? void 0 : err.name)) {
	                        if (!closed)
	                            execInGlobalContext(function () {
	                                if (closed)
	                                    return;
	                                observer.error && observer.error(err);
	                            });
	                    }
	                });
	            };
	            setTimeout(doQuery, 0);
	            return subscription;
	        });
	        observable.hasValue = function () { return hasValue; };
	        observable.getValue = function () { return currentValue; };
	        return observable;
	    }

	    var Dexie = Dexie$1;
	    props(Dexie, __assign(__assign({}, fullNameExceptions), {
	        delete: function (databaseName) {
	            var db = new Dexie(databaseName, { addons: [] });
	            return db.delete();
	        },
	        exists: function (name) {
	            return new Dexie(name, { addons: [] }).open().then(function (db) {
	                db.close();
	                return true;
	            }).catch('NoSuchDatabaseError', function () { return false; });
	        },
	        getDatabaseNames: function (cb) {
	            try {
	                return getDatabaseNames(Dexie.dependencies).then(cb);
	            }
	            catch (_a) {
	                return rejection(new exceptions.MissingAPI());
	            }
	        },
	        defineClass: function () {
	            function Class(content) {
	                extend(this, content);
	            }
	            return Class;
	        }, ignoreTransaction: function (scopeFunc) {
	            return PSD.trans ?
	                usePSD(PSD.transless, scopeFunc) :
	                scopeFunc();
	        }, vip: vip, async: function (generatorFn) {
	            return function () {
	                try {
	                    var rv = awaitIterator(generatorFn.apply(this, arguments));
	                    if (!rv || typeof rv.then !== 'function')
	                        return DexiePromise.resolve(rv);
	                    return rv;
	                }
	                catch (e) {
	                    return rejection(e);
	                }
	            };
	        }, spawn: function (generatorFn, args, thiz) {
	            try {
	                var rv = awaitIterator(generatorFn.apply(thiz, args || []));
	                if (!rv || typeof rv.then !== 'function')
	                    return DexiePromise.resolve(rv);
	                return rv;
	            }
	            catch (e) {
	                return rejection(e);
	            }
	        },
	        currentTransaction: {
	            get: function () { return PSD.trans || null; }
	        }, waitFor: function (promiseOrFunction, optionalTimeout) {
	            var promise = DexiePromise.resolve(typeof promiseOrFunction === 'function' ?
	                Dexie.ignoreTransaction(promiseOrFunction) :
	                promiseOrFunction)
	                .timeout(optionalTimeout || 60000);
	            return PSD.trans ?
	                PSD.trans.waitFor(promise) :
	                promise;
	        },
	        Promise: DexiePromise,
	        debug: {
	            get: function () { return debug; },
	            set: function (value) {
	                setDebug(value);
	            }
	        },
	        derive: derive, extend: extend, props: props, override: override,
	        Events: Events, on: globalEvents, liveQuery: liveQuery, extendObservabilitySet: extendObservabilitySet,
	        getByKeyPath: getByKeyPath, setByKeyPath: setByKeyPath, delByKeyPath: delByKeyPath, shallowClone: shallowClone, deepClone: deepClone, getObjectDiff: getObjectDiff, cmp: cmp, asap: asap$1,
	        minKey: minKey,
	        addons: [],
	        connections: connections,
	        errnames: errnames,
	        dependencies: domDeps, cache: cache,
	        semVer: DEXIE_VERSION, version: DEXIE_VERSION.split('.')
	            .map(function (n) { return parseInt(n); })
	            .reduce(function (p, c, i) { return p + (c / Math.pow(10, i * 2)); }) }));
	    Dexie.maxKey = getMaxKey(Dexie.dependencies.IDBKeyRange);

	    if (typeof dispatchEvent !== 'undefined' && typeof addEventListener !== 'undefined') {
	        globalEvents(DEXIE_STORAGE_MUTATED_EVENT_NAME, function (updatedParts) {
	            if (!propagatingLocally) {
	                var event_1;
	                event_1 = new CustomEvent(STORAGE_MUTATED_DOM_EVENT_NAME, {
	                    detail: updatedParts
	                });
	                propagatingLocally = true;
	                dispatchEvent(event_1);
	                propagatingLocally = false;
	            }
	        });
	        addEventListener(STORAGE_MUTATED_DOM_EVENT_NAME, function (_a) {
	            var detail = _a.detail;
	            if (!propagatingLocally) {
	                propagateLocally(detail);
	            }
	        });
	    }
	    function propagateLocally(updateParts) {
	        var wasMe = propagatingLocally;
	        try {
	            propagatingLocally = true;
	            globalEvents.storagemutated.fire(updateParts);
	            signalSubscribersNow(updateParts, true);
	        }
	        finally {
	            propagatingLocally = wasMe;
	        }
	    }
	    var propagatingLocally = false;

	    var bc;
	    var createBC = function () { };
	    if (typeof BroadcastChannel !== 'undefined') {
	        createBC = function () {
	            bc = new BroadcastChannel(STORAGE_MUTATED_DOM_EVENT_NAME);
	            bc.onmessage = function (ev) { return ev.data && propagateLocally(ev.data); };
	        };
	        createBC();
	        if (typeof bc.unref === 'function') {
	            bc.unref();
	        }
	        globalEvents(DEXIE_STORAGE_MUTATED_EVENT_NAME, function (changedParts) {
	            if (!propagatingLocally) {
	                bc.postMessage(changedParts);
	            }
	        });
	    }

	    if (typeof addEventListener !== 'undefined') {
	        addEventListener('pagehide', function (event) {
	            if (!Dexie$1.disableBfCache && event.persisted) {
	                if (debug)
	                    console.debug('Dexie: handling persisted pagehide');
	                bc === null || bc === void 0 ? void 0 : bc.close();
	                for (var _i = 0, connections_1 = connections; _i < connections_1.length; _i++) {
	                    var db = connections_1[_i];
	                    db.close({ disableAutoOpen: false });
	                }
	            }
	        });
	        addEventListener('pageshow', function (event) {
	            if (!Dexie$1.disableBfCache && event.persisted) {
	                if (debug)
	                    console.debug('Dexie: handling persisted pageshow');
	                createBC();
	                propagateLocally({ all: new RangeSet(-Infinity, [[]]) });
	            }
	        });
	    }

	    function add(value) {
	        return new PropModification({ add: value });
	    }

	    function remove(value) {
	        return new PropModification({ remove: value });
	    }

	    function replacePrefix(a, b) {
	        return new PropModification({ replacePrefix: [a, b] });
	    }

	    DexiePromise.rejectionMapper = mapError;
	    setDebug(debug);

	    var namedExports = /*#__PURE__*/Object.freeze({
	        __proto__: null,
	        Dexie: Dexie$1,
	        liveQuery: liveQuery,
	        Entity: Entity,
	        cmp: cmp,
	        PropModSymbol: PropModSymbol,
	        PropModification: PropModification,
	        replacePrefix: replacePrefix,
	        add: add,
	        remove: remove,
	        'default': Dexie$1,
	        RangeSet: RangeSet,
	        mergeRanges: mergeRanges,
	        rangesOverlap: rangesOverlap
	    });

	    __assign(Dexie$1, namedExports, { default: Dexie$1 });

	    return Dexie$1;

	}));
	
} (dexie));

var dexieExports = dexie.exports;
var _Dexie = /*@__PURE__*/getDefaultExportFromCjs(dexieExports);

// Making the module version consumable via require - to prohibit
// multiple occurrancies of the same module in the same app
// (dual package hazard, https://nodejs.org/api/packages.html#dual-package-hazard)
const DexieSymbol = Symbol.for("Dexie");
const Dexie = globalThis[DexieSymbol] || (globalThis[DexieSymbol] = _Dexie);
if (_Dexie.semVer !== Dexie.semVer) {
    throw new Error(`Two different versions of Dexie loaded in the same app: ${_Dexie.semVer} and ${Dexie.semVer}`);
}

const DATABASE_NAME = 'MidenClientDB';

async function openDatabase() {
  console.log('Opening database...');
  try {
      await db.open();
      console.log("Database opened successfully");
      return true;
  } catch (err) {
      console.error("Failed to open database: ", err);
      return false;
  }
}

const Table = {
  AccountCode: 'accountCode',
  AccountStorage: 'accountStorage',
  AccountVaults: 'accountVaults',
  AccountAuth: 'accountAuth',
  Accounts: 'accounts',
  Transactions: 'transactions',
  TransactionScripts: 'transactionScripts',
  InputNotes: 'inputNotes',
  OutputNotes: 'outputNotes',
  NotesScripts: 'notesScripts',
  StateSync: 'stateSync',
  BlockHeaders: 'blockHeaders',
  ChainMmrNodes: 'chainMmrNodes',
};

const db = new Dexie(DATABASE_NAME);
db.version(1).stores({
  [Table.AccountCode]: indexes('root'),
  [Table.AccountStorage]: indexes('root'),
  [Table.AccountVaults]: indexes('root'),
  [Table.AccountAuth]: indexes('accountId', 'pubKey'),
  [Table.Accounts]: indexes('[id+nonce]', 'codeRoot', 'storageRoot', 'vaultRoot'),
  [Table.Transactions]: indexes('id'),
  [Table.TransactionScripts]: indexes('scriptHash'),
  [Table.InputNotes]: indexes('noteId', 'recipient', 'status'),
  [Table.OutputNotes]: indexes('noteId', 'recipient', 'status'),
  [Table.NotesScripts]: indexes('scriptHash'),
  [Table.StateSync]: indexes('id'),
  [Table.BlockHeaders]: indexes('blockNum'),
  [Table.ChainMmrNodes]: indexes('id'),
});

function indexes(...items) {
  return items.join(',');
}

db.on('populate', () => {
  // Populate the stateSync table with default values
  db.stateSync.put({ id: 1, blockNum: "0", tags: [] });
});

const accountCodes = db.table(Table.AccountCode);
const accountStorages = db.table(Table.AccountStorage);
const accountVaults = db.table(Table.AccountVaults);
const accountAuths = db.table(Table.AccountAuth);
const accounts = db.table(Table.Accounts);
const transactions = db.table(Table.Transactions);
const transactionScripts = db.table(Table.TransactionScripts);
const inputNotes = db.table(Table.InputNotes);
const outputNotes = db.table(Table.OutputNotes);
const notesScripts = db.table(Table.NotesScripts);
const stateSync = db.table(Table.StateSync);
const blockHeaders = db.table(Table.BlockHeaders);
const chainMmrNodes = db.table(Table.ChainMmrNodes);

// GET FUNCTIONS
async function getAccountIds() {
    try {
        let allIds = new Set(); // Use a Set to ensure uniqueness

        // Iterate over each account entry
        await accounts.each(account => {
            allIds.add(account.id); // Assuming 'account' has an 'id' property
        });

        return Array.from(allIds); // Convert back to array to return a list of unique IDs
    } catch (error) {
        console.error("Failed to retrieve account IDs: ", error);
        throw error; // Or handle the error as fits your application's error handling strategy
    }
}

async function getAllAccountStubs() {
    try {
        // Fetch all records
        const allRecords = await accounts.toArray();
        
        // Use a Map to track the latest record for each id based on nonce
        const latestRecordsMap = new Map();

        allRecords.forEach(record => {
            const existingRecord = latestRecordsMap.get(record.id);
            if (!existingRecord || BigInt(record.nonce) > BigInt(existingRecord.nonce)) {
                latestRecordsMap.set(record.id, record);
            }
        });

        // Extract the latest records from the Map
        const latestRecords = Array.from(latestRecordsMap.values());

        const resultObject = await Promise.all(latestRecords.map(async record => {
            let accountSeedBase64 = null;
            if (record.accountSeed) {
                // Ensure accountSeed is processed as a Uint8Array and converted to Base64
                let accountSeedArrayBuffer = await record.accountSeed.arrayBuffer();
                let accountSeedArray = new Uint8Array(accountSeedArrayBuffer);
                accountSeedBase64 = uint8ArrayToBase64$2(accountSeedArray);
            }

            return {
                id: record.id,
                nonce: record.nonce,
                vault_root: record.vaultRoot,
                storage_root: record.storageRoot,
                code_root: record.codeRoot,
                account_seed: accountSeedBase64 // Now correctly formatted as Base64
            };
        }));

        return resultObject;
    } catch (error) {
        console.error('Error fetching all latest account stubs:', error);
        throw error;
    }
}

async function getAccountStub(
    accountId
) {
    try {
        let allRecords = await accounts.toArray();
        // Fetch all records matching the given id
        const allMatchingRecords = await accounts
          .where('id')
          .equals(accountId)
          .toArray();
    
        if (allMatchingRecords.length === 0) {
          console.log('No records found for given ID.');
          return null; // No records found
        }
    
        // Convert nonce to BigInt and sort
        // Note: This assumes all nonces are valid BigInt strings.
        const sortedRecords = allMatchingRecords.sort((a, b) => {
          const bigIntA = BigInt(a.nonce);
          const bigIntB = BigInt(b.nonce);
          return bigIntA > bigIntB ? -1 : bigIntA < bigIntB ? 1 : 0;
        });
    
        // The first record is the most recent one due to the sorting
        const mostRecentRecord = sortedRecords[0];

        let accountSeedBase64 = null;
        if (mostRecentRecord.accountSeed) {
            // Ensure accountSeed is processed as a Uint8Array and converted to Base64
            let accountSeedArrayBuffer = await mostRecentRecord.accountSeed.arrayBuffer();
            let accountSeedArray = new Uint8Array(accountSeedArrayBuffer);
            accountSeedBase64 = uint8ArrayToBase64$2(accountSeedArray);
        }
        const accountStub = {
            id: mostRecentRecord.id,
            nonce: mostRecentRecord.nonce,
            vault_root: mostRecentRecord.vaultRoot,
            storage_root: mostRecentRecord.storageRoot,
            code_root: mostRecentRecord.codeRoot,
            account_seed: accountSeedBase64
        };
        return accountStub;
      } catch (error) {
        console.error('Error fetching most recent account record:', error);
        throw error; // Re-throw the error for further handling
      }
}

async function getAccountCode(
    codeRoot
) {
    try {
        // Fetch all records matching the given root
        const allMatchingRecords = await accountCodes
            .where('root')
            .equals(codeRoot)
            .toArray();

        if (allMatchingRecords.length === 0) {
            console.log('No records found for given code root.');
            return null; // No records found
        }

        // The first record is the only one due to the uniqueness constraint
        const codeRecord = allMatchingRecords[0];

        // Convert the module Blob to an ArrayBuffer
        const moduleArrayBuffer = await codeRecord.module.arrayBuffer();
        const moduleArray = new Uint8Array(moduleArrayBuffer);
        const moduleBase64 = uint8ArrayToBase64$2(moduleArray);
        return {
            root: codeRecord.root,
            procedures: codeRecord.procedures,
            module: moduleBase64,
        };
    } catch (error) {
        console.error('Error fetching code record:', error);
        throw error; // Re-throw the error for further handling
    }
}

async function getAccountStorage(
    storageRoot
) {
    try {
        // Fetch all records matching the given root
        const allMatchingRecords = await accountStorages
            .where('root')
            .equals(storageRoot)
            .toArray();

        if (allMatchingRecords.length === 0) {
            console.log('No records found for given storage root.');
            return null; // No records found
        }

        // The first record is the only one due to the uniqueness constraint
        const storageRecord = allMatchingRecords[0];

        // Convert the module Blob to an ArrayBuffer
        const storageArrayBuffer = await storageRecord.slots.arrayBuffer();
        const storageArray = new Uint8Array(storageArrayBuffer);
        const storageBase64 = uint8ArrayToBase64$2(storageArray);
        return {
            root: storageRecord.root,
            storage: storageBase64
        };
    } catch (error) {
        console.error('Error fetching code record:', error);
        throw error; // Re-throw the error for further handling
    }
}

async function getAccountAssetVault(
    vaultRoot
) {
    try {
        // Fetch all records matching the given root
        const allMatchingRecords = await accountVaults
            .where('root')
            .equals(vaultRoot)
            .toArray();

        if (allMatchingRecords.length === 0) {
            console.log('No records found for given vault root.');
            return null; // No records found
        }

        // The first record is the only one due to the uniqueness constraint
        const vaultRecord = allMatchingRecords[0];

        return {
            root: vaultRecord.root,
            assets: vaultRecord.assets
        };
    } catch (error) {
        console.error('Error fetching code record:', error);
        throw error; // Re-throw the error for further handling
    }
}

async function getAccountAuth(
    accountId
) {
    try {
        // Fetch all records matching the given id
        const allMatchingRecords = await accountAuths
            .where('accountId')
            .equals(accountId)
            .toArray();

        if (allMatchingRecords.length === 0) {
            console.log('No records found for given account ID.');
            return null; // No records found
        }

        // The first record is the only one due to the uniqueness constraint
        const authRecord = allMatchingRecords[0];

        // Convert the authInfo Blob to an ArrayBuffer
        const authInfoArrayBuffer = await authRecord.authInfo.arrayBuffer();
        const authInfoArray = new Uint8Array(authInfoArrayBuffer);
        const authInfoBase64 = uint8ArrayToBase64$2(authInfoArray);

        return {
            id: authRecord.accountId,
            auth_info: authInfoBase64
        };
    } catch (err) {
        console.error('Error fetching account auth:', err);
        throw err; // Re-throw the error for further handling
    }
}

function getAccountAuthByPubKey(
    pubKey
) {
    // Try to get the account auth from the cache
    let pubKeyArray = new Uint8Array(pubKey);
    let pubKeyBase64 = uint8ArrayToBase64$2(pubKeyArray);
    let cachedAccountAuth = ACCOUNT_AUTH_MAP.get(pubKeyBase64);

    // If it's not in the cache, throw an error
    if (!cachedAccountAuth) {
        throw new Error('Account auth not found in cache.');
    }

    let data = {
        id: cachedAccountAuth.id,
        auth_info: cachedAccountAuth.auth_info
    };

    return data;
}

var ACCOUNT_AUTH_MAP = new Map();
async function fetchAndCacheAccountAuthByPubKey(
    accountId
) {
    try {
        // Fetch all records matching the given id
        const allMatchingRecords = await accountAuths
            .where('accountId')
            .equals(accountId)
            .toArray();

        if (allMatchingRecords.length === 0) {
            console.log('No records found for given account ID.');
            return null; // No records found
        }

        // The first record is the only one due to the uniqueness constraint
        const authRecord = allMatchingRecords[0];

        // Convert the authInfo Blob to an ArrayBuffer
        const authInfoArrayBuffer = await authRecord.authInfo.arrayBuffer();
        const authInfoArray = new Uint8Array(authInfoArrayBuffer);
        const authInfoBase64 = uint8ArrayToBase64$2(authInfoArray);

        // Store the auth info in the map
        ACCOUNT_AUTH_MAP.set(authRecord.pubKey, {
            id: authRecord.accountId,
            auth_info: authInfoBase64
        });

        return {
            id: authRecord.accountId,
            auth_info: authInfoBase64
        };
    } catch (err) {
        console.error('Error fetching account auth by public key:', err);
        throw err; // Re-throw the error for further handling
    }
}

// INSERT FUNCTIONS

async function insertAccountCode(
    codeRoot, 
    code, 
    module
) {
    try {
        // Create a Blob from the ArrayBuffer
        const moduleBlob = new Blob([new Uint8Array(module)]);

        // Prepare the data object to insert
        const data = {
            root: codeRoot, // Using codeRoot as the key
            procedures: code,
            module: moduleBlob, // Blob created from ArrayBuffer
        };

        // Perform the insert using Dexie
        await accountCodes.add(data);
    } catch (error) {
        console.error(`Error inserting code with root: ${codeRoot}:`, error);
        throw error; // Rethrow the error to handle it further up the call chain if needed
    }
}

async function insertAccountStorage(
    storageRoot, 
    storageSlots
) {
    try {
        const storageSlotsBlob = new Blob([new Uint8Array(storageSlots)]);

        // Prepare the data object to insert
        const data = {
            root: storageRoot, // Using storageRoot as the key
            slots: storageSlotsBlob, // Blob created from ArrayBuffer
        };

        // Perform the insert using Dexie
        await accountStorages.add(data);
    } catch (error) {
        console.error(`Error inserting storage with root: ${storageRoot}:`, error);
        throw error; // Rethrow the error to handle it further up the call chain if needed
    }
}

async function insertAccountAssetVault(
    vaultRoot, 
    assets
) {
    try {
        // Prepare the data object to insert
        const data = {
            root: vaultRoot, // Using vaultRoot as the key
            assets: assets,
        };

        // Perform the insert using Dexie
        await accountVaults.add(data);
    } catch (error) {
        console.error(`Error inserting vault with root: ${vaultRoot}:`, error);
        throw error; // Rethrow the error to handle it further up the call chain if needed
    }
}

async function insertAccountRecord(
    accountId,
    code_root,
    storage_root,
    vault_root,
    nonce,
    committed,
    account_seed
) {
    try {
        let accountSeedBlob = null;
        if (account_seed) {
            accountSeedBlob = new Blob([new Uint8Array(account_seed)]);
        }
        

        // Prepare the data object to insert
        const data = {
            id: accountId, // Using accountId as the key
            codeRoot: code_root,
            storageRoot: storage_root,
            vaultRoot: vault_root,
            nonce: nonce,
            committed: committed,
            accountSeed: accountSeedBlob,
        };

        // Perform the insert using Dexie
        await accounts.add(data);
    } catch (error) {
        console.error(`Error inserting account: ${accountId}:`, error);
        throw error; // Rethrow the error to handle it further up the call chain if needed
    }
}

async function insertAccountAuth(
    accountId, 
    authInfo,
    pubKey
) {
    try {
        let authInfoBlob = new Blob([new Uint8Array(authInfo)]);
        let pubKeyArray = new Uint8Array(pubKey);
        let pubKeyBase64 = uint8ArrayToBase64$2(pubKeyArray);

        // Prepare the data object to insert
        const data = {
            accountId: accountId, // Using accountId as the key
            authInfo: authInfoBlob,
            pubKey: pubKeyBase64
        };

        // Perform the insert using Dexie
        await accountAuths.add(data);
    } catch (error) {
        console.error(`Error inserting auth for account: ${accountId}:`, error);
        throw error; // Rethrow the error to handle it further up the call chain if needed
    }
}

function uint8ArrayToBase64$2(bytes) {
    const binary = bytes.reduce((acc, byte) => acc + String.fromCharCode(byte), '');
    return btoa(binary);
}

// INSERT FUNCTIONS
async function insertBlockHeader(
    blockNum,
    header,
    chainMmrPeaks,
    hasClientNotes
) {
    try {
        const data = {
            blockNum: blockNum,
            header: header,
            chainMmrPeaks: chainMmrPeaks,
            hasClientNotes: hasClientNotes
        };

        await blockHeaders.add(data);
    } catch (err) {
        console.error("Failed to insert block header: ", err);
        throw err;
    }
}

async function insertChainMmrNodes(
    ids,
    nodes
) {
    try {
        const data = nodes.map((node, index) => {
            return {
                id: ids[index],
                node: node
            }
        });

        await chainMmrNodes.bulkAdd(data);
    } catch (err) {
        console.error("Failed to insert chain mmr nodes: ", err);
        throw err;
    }
}

// GET FUNCTIONS
async function getBlockHeaders(
    blockNumbers
) {
    try {
        const blockHeaderPromises = blockNumbers.map(blockNum => 
            blockHeaders.get(blockNum)
        );

        const results = await Promise.all(blockHeaderPromises);
        
        results.forEach((result, index) => {
            if (result === undefined) {
                results[index] = null;
            } else {
                results[index] = {
                    block_num: results[index].blockNum,
                    header: results[index].header,
                    chain_mmr: results[index].chainMmrPeaks,
                    has_client_notes: results[index].hasClientNotes
                };
            }
        });

        return results
    } catch (err) {
        console.error("Failed to get block headers: ", err);
        throw err;
    }
}

async function getChainMmrPeaksByBlockNum(
    blockNum
) {
    try {
        const blockHeader = await blockHeaders.get(blockNum);
        return {
            peaks: blockHeader.chainMmrPeaks
        };
    } catch (err) {
        console.error("Failed to get chain mmr peaks: ", err);
        throw err;
    }
}

async function getChainMmrNodesAll() {
    try {
        const chainMmrNodesAll = await chainMmrNodes.toArray();
        return chainMmrNodesAll;
    } catch (err) {
        console.error("Failed to get chain mmr nodes: ", err);
        throw err;
    }
}

async function getChainMmrNodes(
    ids
) {
    try {
        const chainMmrNodesPromises = ids.map(id =>
            chainMmrNodes.get(id)
        );

        const results = await Promise.all(chainMmrNodesPromises);
        return results;
    } catch (err) {
        console.error("Failed to get chain mmr nodes: ", err);
        throw err;
    }
}

async function getOutputNotes(
    status
) {
    try {
        let notes;

        // Fetch the records based on the filter
        if (status === 'All') {
            notes = await outputNotes.toArray();
        } else {
            notes = await outputNotes.where('status').equals(status).toArray();
        }

        return await processOutputNotes(notes);
    } catch (err) {
        console.error("Failed to get input notes: ", err);
        throw err;
    }
}

async function getInputNotes(
    status
) {
    try {
        let notes;

        // Fetch the records based on the filter
        if (status === 'All') {
            notes = await inputNotes.toArray();
        } else {
            notes = await inputNotes.where('status').equals(status).toArray();
        }

        return await processInputNotes(notes);
    } catch (err) {
        console.error("Failed to get input notes: ", err);
        throw err;
    }
}

async function getInputNotesFromIds(
    noteIds
) {
    try {
        let notes;

        // Fetch the records based on a list of IDs
        notes = await inputNotes.where('noteId').anyOf(noteIds).toArray();

        return await processInputNotes(notes);
    } catch (err) {
        console.error("Failed to get input notes: ", err);
        throw err;
    }
}

async function getOutputNotesFromIds(
    noteIds
) {
    try {
        let notes;

        // Fetch the records based on a list of IDs
        notes = await outputNotes.where('noteId').anyOf(noteIds).toArray();

        return await processOutputNotes(notes);
    } catch (err) {
        console.error("Failed to get input notes: ", err);
        throw err;
    }
}

async function getUnspentInputNoteNullifiers() {
    try {
        const notes = await inputNotes
            .where('status')
            .anyOf(['Committed', 'Processing'])
            .toArray();
        const nullifiers = notes.map(note => JSON.parse(note.details).nullifier);

        return nullifiers;
    } catch (err) {
        console.error("Failed to get unspent input note nullifiers: ", err);
        throw err;
    }
}

async function insertInputNote(
    noteId,
    assets,
    recipient,
    status,
    metadata,
    details,
    noteScriptHash,
    serializedNoteScript,
    inclusionProof,
    serializedCreatedAt
) {
    return db.transaction('rw', inputNotes, notesScripts, async (tx) => {
        try {
            let assetsBlob = new Blob([new Uint8Array(assets)]);

            // Prepare the data object to insert
            const data = {
                noteId: noteId,
                assets: assetsBlob,
                recipient: recipient,
                status: status,
                metadata: metadata ? metadata : null,
                details: details,
                inclusionProof: inclusionProof ? JSON.stringify(inclusionProof) : null,
                consumerTransactionId: null,
                createdAt: serializedCreatedAt
            };

            // Perform the insert using Dexie
            await tx.inputNotes.add(data);

            let serializedNoteScriptBlob = new Blob([new Uint8Array(serializedNoteScript)]);

            const noteScriptData = {
                scriptHash: noteScriptHash,
                serializedNoteScript: serializedNoteScriptBlob,
            };

            await tx.notesScripts.put(noteScriptData);
        } catch {
            console.error(`Error inserting note: ${noteId}:`, error);
            throw error; // Rethrow the error to handle it further up the call chain if needed
        }
    });
}

async function insertOutputNote(
    noteId,
    assets,
    recipient,
    status,
    metadata,
    details,
    noteScriptHash,
    serializedNoteScript,
    inclusionProof,
    serializedCreatedAt
) {
    return db.transaction('rw', outputNotes, notesScripts, async (tx) => {
        try {
            let assetsBlob = new Blob([new Uint8Array(assets)]);

            // Prepare the data object to insert
            const data = {
                noteId: noteId,
                assets: assetsBlob,
                recipient: recipient,
                status: status,
                metadata: metadata,
                details: details ? details : null,
                inclusionProof: inclusionProof ? JSON.stringify(inclusionProof) : null,
                consumerTransactionId: null,
                createdAt: serializedCreatedAt
            };

            // Perform the insert using Dexie
            await tx.outputNotes.add(data);

            if (noteScriptHash) {
                const exists = await tx.notesScripts.get(noteScriptHash);
                if (!exists) {
                    let serializedNoteScriptBlob = null;
                    if (serializedNoteScript) {
                        serializedNoteScriptBlob = new Blob([new Uint8Array(serializedNoteScript)]);
                    }

                    const data = {
                        scriptHash: noteScriptHash,
                        serializedNoteScript: serializedNoteScriptBlob,
                    };
                    await tx.notesScripts.add(data);
                }
            }
        } catch {
            console.error(`Error inserting note: ${noteId}:`, error);
            throw error; // Rethrow the error to handle it further up the call chain if needed
        }
    });
}

async function updateNoteConsumerTxId(noteId, consumerTxId, submittedAt) {
    try {
        // Start a transaction that covers both tables
        await db.transaction('rw', inputNotes, outputNotes, async (tx) => {
            // Update input_notes where note_id matches
            const updatedInputNotes = await tx.inputNotes
                .where('noteId')
                .equals(noteId)
                .modify({ consumerTransactionId: consumerTxId, submittedAt: submittedAt, status: "Processing" });

            // Update output_notes where note_id matches
            const updatedOutputNotes = await tx.outputNotes
                .where('noteId')
                .equals(noteId)
                .modify({ consumerTransactionId: consumerTxId, submittedAt: submittedAt, status: "Processing" });

            // Log the count of updated entries in both tables (optional)
            console.log(`Updated ${updatedInputNotes} input notes and ${updatedOutputNotes} output notes`);
        });
    } catch (err) {
        console.error("Failed to update note consumer transaction ID: ", err);
        throw err;
    }
}

async function processInputNotes(
    notes
) {
    // Fetch all scripts from the scripts table for joining
    const scripts = await notesScripts.toArray();
    const scriptMap = new Map(scripts.map(script => [script.scriptHash, script.serializedNoteScript]));

    const transactionRecords = await transactions.toArray();
    const transactionMap = new Map(transactionRecords.map(transaction => [transaction.id, transaction.accountId]));

    const processedNotes = await Promise.all(notes.map(async note => {
        // Convert the assets blob to base64
        const assetsArrayBuffer = await note.assets.arrayBuffer();
        const assetsArray = new Uint8Array(assetsArrayBuffer);
        const assetsBase64 = uint8ArrayToBase64$1(assetsArray);
        note.assets = assetsBase64;

        // Convert the serialized note script blob to base64
        let serializedNoteScriptBase64 = null;
        // Parse details JSON and perform a "join"
        if (note.details) {
            const details = JSON.parse(note.details);
            if (details.script_hash) {
                let serializedNoteScript = scriptMap.get(details.script_hash);
                let serializedNoteScriptArrayBuffer = await serializedNoteScript.arrayBuffer();
                const serializedNoteScriptArray = new Uint8Array(serializedNoteScriptArrayBuffer);
                serializedNoteScriptBase64 = uint8ArrayToBase64$1(serializedNoteScriptArray);
            }
        }

        // Perform a "join" with the transactions table
        let consumerAccountId = null;
        if (transactionMap.has(note.consumerTransactionId)) { 
            consumerAccountId = transactionMap.get(note.consumerTransactionId);
        }

        return {
            assets: note.assets,
            details: note.details,
            recipient: note.recipient,
            status: note.status,
            metadata: note.metadata ? note.metadata : null,
            inclusion_proof: note.inclusionProof ? note.inclusionProof : null,
            serialized_note_script: serializedNoteScriptBase64,
            consumer_account_id: consumerAccountId,
            created_at: note.createdAt,
            submitted_at: note.submittedAt ? note.submittedAt : null,
            nullifier_height: note.nullifierHeight ? note.nullifierHeight : null
        };
    }));
    return processedNotes;
}

async function processOutputNotes(
    notes
) {
    // Fetch all scripts from the scripts table for joining
    const scripts = await notesScripts.toArray();
    const scriptMap = new Map(scripts.map(script => [script.scriptHash, script.serializedNoteScript]));

    const transactionRecords = await transactions.toArray();
    const transactionMap = new Map(transactionRecords.map(transaction => [transaction.id, transaction.accountId]));

    // Process each note to convert 'blobField' from Blob to Uint8Array
    const processedNotes = await Promise.all(notes.map(async note => {
        const assetsArrayBuffer = await note.assets.arrayBuffer();
        const assetsArray = new Uint8Array(assetsArrayBuffer);
        const assetsBase64 = uint8ArrayToBase64$1(assetsArray);
        note.assets = assetsBase64;

        let serializedNoteScriptBase64 = null;
        // Parse details JSON and perform a "join"
        if (note.details) {
            const details = JSON.parse(note.details);
            if (details.script_hash) {
                let serializedNoteScript = scriptMap.get(details.script_hash);
                let serializedNoteScriptArrayBuffer = await serializedNoteScript.arrayBuffer();
                const serializedNoteScriptArray = new Uint8Array(serializedNoteScriptArrayBuffer);
                serializedNoteScriptBase64 = uint8ArrayToBase64$1(serializedNoteScriptArray);
            }
        }

        // Perform a "join" with the transactions table
        let consumerAccountId = null;
        if (transactionMap.has(note.consumerTransactionId)) { 
            consumerAccountId = transactionMap.get(note.consumerTransactionId);
        }

        return {
            assets: note.assets,
            details: note.details ? note.details : null,
            recipient: note.recipient,
            status: note.status,
            metadata: note.metadata,
            inclusion_proof: note.inclusionProof ? note.inclusionProof : null,
            serialized_note_script: serializedNoteScriptBase64,
            consumer_account_id: consumerAccountId,
            created_at: note.createdAt,
            submitted_at: note.submittedAt ? note.submittedAt : null,
            nullifier_height: note.nullifierHeight ? note.nullifierHeight : null
        };
    }));
    return processedNotes;
}

function uint8ArrayToBase64$1(bytes) {
    const binary = bytes.reduce((acc, byte) => acc + String.fromCharCode(byte), '');
    return btoa(binary);
}

async function getNoteTags() {
    try {
        const record = await stateSync.get(1);  // Since id is the primary key and always 1
        if (record) {
            let data = null;
            if (record.tags.length === 0) {
                data = {
                    tags: JSON.stringify(record.tags)
                };
            } else {
                data = {
                    tags: record.tags
                };
            };
            return data;
        } else {
            return null;
        }
    } catch (error) {
        console.error('Error fetching record:', error);
        return null;
    }
}

async function getSyncHeight() {
    try {
        const record = await stateSync.get(1);  // Since id is the primary key and always 1
        if (record) {
            let data = {
                block_num: record.blockNum
            };
            return data;
        } else {
            return null;
        }
    } catch (error) {
        console.error('Error fetching record:', error);
        return null;
    }
}

async function addNoteTag(
    tags
) {
    try {
        await stateSync.update(1, { tags: tags });
    } catch {
        console.error("Failed to add note tag: ", err);
        throw err;
    }
}

async function applyStateSync(
    blockNum,
    nullifiers,
    nullifierBlockNums,
    blockHeader,
    chainMmrPeaks,
    hasClientNotes,
    nodeIndices,
    nodes,
    outputNoteIds,
    outputNoteInclusionProofs,
    inputNoteIds,
    inputNoteInluclusionProofs,
    inputeNoteMetadatas,
    transactionIds,
    transactionBlockNums
) {
    return db.transaction('rw', stateSync, inputNotes, outputNotes, transactions, blockHeaders, chainMmrNodes, async (tx) => {
        await updateSyncHeight(tx, blockNum);
        await updateSpentNotes(tx, nullifierBlockNums, nullifiers);
        await updateBlockHeader(tx, blockNum, blockHeader, chainMmrPeaks, hasClientNotes);
        await updateChainMmrNodes(tx, nodeIndices, nodes);
        await updateCommittedNotes(tx, outputNoteIds, outputNoteInclusionProofs, inputNoteIds, inputNoteInluclusionProofs, inputeNoteMetadatas);
        await updateCommittedTransactions(tx, transactionBlockNums, transactionIds);
    });
}

async function updateSyncHeight(
    tx, 
    blockNum
) {
    try {
        await tx.stateSync.update(1, { blockNum: blockNum });
    } catch (error) {
        console.error("Failed to update sync height: ", error);
        throw error;
    }
}

async function updateSpentNotes(
    tx,
    nullifierBlockNums,
    nullifiers
) {
    try {
        // Fetch all notes
        const inputNotes = await tx.inputNotes.toArray();
        const outputNotes = await tx.outputNotes.toArray();

        // Pre-parse all details and store them with their respective note ids for quick access
        const parsedInputNotes = inputNotes.map(note => ({
            noteId: note.noteId,
            details: JSON.parse(note.details)  // Parse the JSON string into an object
        }));

        // Iterate through each parsed note and check against the list of nullifiers
        for (const note of parsedInputNotes) {
            if (note.details && note.details.nullifier) {
                const nullifierIndex = nullifiers.indexOf(note.details.nullifier);
                if (nullifierIndex !== -1) {
                    // If the nullifier is in the list, update the note's status and set nullifierHeight to the index
                    await tx.inputNotes.update(note.noteId, { status: 'Consumed', nullifierHeight: nullifierBlockNums[nullifierIndex] });
                }
            }
        }

         // Pre-parse all details and store them with their respective note ids for quick access
         const parsedOutputNotes = outputNotes.map(note => ({
            noteId: note.noteId,
            details: JSON.parse(note.details)  // Parse the JSON string into an object
        }));

        // Iterate through each parsed note and check against the list of nullifiers
        for (const note of parsedOutputNotes) {
            if (note.details && note.details.nullifier) {
                const nullifierIndex = nullifiers.indexOf(note.details.nullifier);
                if (nullifierIndex !== -1) {
                    // If the nullifier is in the list, update the note's status and set nullifierHeight to the index
                    await tx.outputNotes.update(note.noteId, { status: 'Consumed', nullifierHeight: nullifierBlockNums[nullifierIndex] });
                }
            }
        }
    } catch (error) {
        console.error("Error updating input notes:", error);
        throw error;
    }
}

async function updateBlockHeader(
    tx,
    blockNum, 
    blockHeader,
    chainMmrPeaks,
    hasClientNotes
) {
    try {
        const data = {
            blockNum: blockNum,
            header: blockHeader,
            chainMmrPeaks: chainMmrPeaks,
            hasClientNotes: hasClientNotes
        };

        await tx.blockHeaders.add(data);
    } catch (err) {
        console.error("Failed to insert block header: ", err);
        throw error;
    }
}

async function updateChainMmrNodes(
    tx,
    nodeIndices,
    nodes
) {
    try {
        // Check if the arrays are not of the same length
        if (nodeIndices.length !== nodes.length) {
            throw new Error("nodeIndices and nodes arrays must be of the same length");
        }

        if (nodeIndices.length === 0) {
            return;
        }

        // Create the updates array with objects matching the structure expected by your IndexedDB schema
        const updates = nodeIndices.map((index, i) => ({
            id: index,  // Assuming 'index' is the primary key or part of it
            node: nodes[i] // Other attributes of the object
        }));

        // Perform bulk update or insertion; assumes tx.chainMmrNodes is a valid table reference in a transaction
        await tx.chainMmrNodes.bulkAdd(updates);
    } catch (err) {
        console.error("Failed to update chain mmr nodes: ", err);
        throw error;
    }
}

async function updateCommittedNotes(
    tx, 
    outputNoteIds, 
    outputNoteInclusionProofs,
    inputNoteIds,
    inputNoteInclusionProofs,
    inputNoteMetadatas
) {
    try {
        if (outputNoteIds.length !== outputNoteInclusionProofs.length) {
            throw new Error("Arrays outputNoteIds and outputNoteInclusionProofs must be of the same length");
        }

        if (
            inputNoteIds.length !== inputNoteInclusionProofs.length && 
            inputNoteIds.length !== inputNoteMetadatas.length && 
            inputNoteInclusionProofs.length !== inputNoteMetadatas.length
        ) {
            throw new Error("Arrays inputNoteIds and inputNoteInclusionProofs and inputNoteMetadatas must be of the same length");
        }

        for (let i = 0; i < outputNoteIds.length; i++) {
            const noteId = outputNoteIds[i];
            const inclusionProof = outputNoteInclusionProofs[i];

            // Update output notes
            await tx.outputNotes.where({ noteId: noteId }).modify({
                status: 'Committed',
                inclusionProof: inclusionProof
            });
        }

        for (let i = 0; i < inputNoteIds.length; i++) {
            const noteId = inputNoteIds[i];
            const inclusionProof = inputNoteInclusionProofs[i];
            const metadata = inputNoteMetadatas[i];

            // Update input notes
            await tx.inputNotes.where({ noteId: noteId }).modify({
                status: 'Committed',
                inclusionProof: inclusionProof,
                metadata: metadata
            });
        }
    } catch (error) {
        console.error("Error updating committed notes:", error);
        throw error;
    }
}

async function updateCommittedTransactions(
    tx, 
    blockNums, 
    transactionIds
) {
    try {
        if (transactionIds.length === 0) {
            return;
        }

        // Fetch existing records
        const existingRecords = await tx.transactions.where('id').anyOf(transactionIds).toArray();

        // Create a mapping of transaction IDs to block numbers
        const transactionBlockMap = transactionIds.reduce((map, id, index) => {
            map[id] = blockNums[index];
            return map;
        }, {});

        // Create updates by merging existing records with the new values
        const updates = existingRecords.map(record => ({
            ...record, // Spread existing fields
            commitHeight: transactionBlockMap[record.id] // Update specific field
        }));

        // Perform the update
        await tx.transactions.bulkPut(updates);
    } catch (err) {
        console.error("Failed to mark transactions as committed: ", err);
        throw err;
    }
}

async function getTransactions(
    filter
) {
    let transactionRecords;

    try {
        if (filter === 'Uncomitted') {
            transactionRecords = await transactions.filter(tx => tx.commitHeight === undefined || tx.commitHeight === null).toArray();
        } else {
            transactionRecords = await transactions.toArray();
        }

        if (transactionRecords.length === 0) {
            return [];
        }

        const scriptHashes = transactionRecords.map(transactionRecord => {
            return transactionRecord.scriptHash
        });

        const scripts = await transactionScripts.where("scriptHash").anyOf(scriptHashes).toArray();

        // Create a map of scriptHash to script for quick lookup
        const scriptMap = new Map();
        scripts.forEach(script => {
            scriptMap.set(script.scriptHash, script.program);
        });

        const processedTransactions = await Promise.all(transactionRecords.map(async transactionRecord => {
            let scriptProgramBase64 = null;

            if (transactionRecord.scriptHash) {
                const scriptProgram = scriptMap.get(transactionRecord.scriptHash);

                if (scriptProgram) {
                    let scriptProgramArrayBuffer = await scriptProgram.arrayBuffer();
                    let scriptProgramArray = new Uint8Array(scriptProgramArrayBuffer);
                    scriptProgramBase64 = uint8ArrayToBase64(scriptProgramArray);
                }
            }

            let outputNotesArrayBuffer = await transactionRecord.outputNotes.arrayBuffer();
            let outputNotesArray = new Uint8Array(outputNotesArrayBuffer);
            let outputNotesBase64 = uint8ArrayToBase64(outputNotesArray);

            transactionRecord.outputNotes = outputNotesBase64;

            let data = {
                id: transactionRecord.id,
                account_id: transactionRecord.accountId,
                init_account_state: transactionRecord.initAccountState,
                final_account_state: transactionRecord.finalAccountState,
                input_notes: transactionRecord.inputNotes,
                output_notes: transactionRecord.outputNotes,
                script_hash: transactionRecord.scriptHash ? transactionRecord.scriptHash : null,
                script_program: scriptProgramBase64,
                script_inputs: transactionRecord.scriptInputs ? transactionRecord.scriptInputs : null,
                block_num: transactionRecord.blockNum,
                commit_height: transactionRecord.commitHeight ? transactionRecord.commitHeight : null
            };

            return data;
        }));

        return processedTransactions
    } catch {
        console.error("Failed to get transactions: ", err);
        throw err;
    }
}

async function insertTransactionScript(
    scriptHash,
    scriptProgram
) {
    try {
        // check if script hash already exists 
        let record = await transactionScripts.where("scriptHash").equals(scriptHash).first();

        if (record) {
            return;
        }

        if (!scriptHash) {
            throw new Error("Script hash must be provided");
        }

        let scriptHashArray = new Uint8Array(scriptHash);
        let scriptHashBase64 = uint8ArrayToBase64(scriptHashArray);
        let scriptProgramBlob = null;

        if (scriptProgram ) {
            scriptProgramBlob = new Blob([new Uint8Array(scriptProgram)]);
        }

        const data = {
            scriptHash: scriptHashBase64,
            program: scriptProgramBlob
        };

        await transactionScripts.add(data);
    } catch (error) {
        // Check if the error is because the record already exists
        if (error.name === 'ConstraintError') ; else {
            // Re-throw the error if it's not a constraint error
            throw error;
        }
    }
}

async function insertProvenTransactionData(
    transactionId,
    accountId,
    initAccountState,
    finalAccountState,
    inputNotes,
    outputNotes,
    scriptHash,
    scriptInputs,
    blockNum,
    committed
) {
    try {
        let scriptHashBase64 = null;
        let outputNotesBlob = new Blob([new Uint8Array(outputNotes)]);
        if (scriptHash !== null) {
            let scriptHashArray = new Uint8Array(scriptHash);
            scriptHashBase64 = uint8ArrayToBase64(scriptHashArray);
        }

        const data = {
            id: transactionId,
            accountId: accountId,
            initAccountState: initAccountState,
            finalAccountState: finalAccountState,
            inputNotes: inputNotes,
            outputNotes: outputNotesBlob,
            scriptHash: scriptHashBase64,
            scriptInputs: scriptInputs ? scriptInputs : null,
            blockNum: blockNum,
            commitHeight: committed ? committed : null
        };

        await transactions.add(data);
    } catch (err) {
        console.error("Failed to insert proven transaction data: ", err);
        throw err;
    }
}

function uint8ArrayToBase64(bytes) {
    const binary = bytes.reduce((acc, byte) => acc + String.fromCharCode(byte), '');
    return btoa(binary);
}

let wasm;

const cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : { decode: () => { throw Error('TextDecoder not available') } } );

if (typeof TextDecoder !== 'undefined') { cachedTextDecoder.decode(); }
let cachedUint8Memory0 = null;

function getUint8Memory0() {
    if (cachedUint8Memory0 === null || cachedUint8Memory0.byteLength === 0) {
        cachedUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8Memory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

const heap = new Array(128).fill(undefined);

heap.push(undefined, null, true, false);

let heap_next = heap.length;

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

function getObject(idx) { return heap[idx]; }

let WASM_VECTOR_LEN = 0;

const cachedTextEncoder = (typeof TextEncoder !== 'undefined' ? new TextEncoder('utf-8') : { encode: () => { throw Error('TextEncoder not available') } } );

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8Memory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8Memory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8Memory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

let cachedInt32Memory0 = null;

function getInt32Memory0() {
    if (cachedInt32Memory0 === null || cachedInt32Memory0.byteLength === 0) {
        cachedInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32Memory0;
}

function dropObject(idx) {
    if (idx < 132) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

let cachedFloat64Memory0 = null;

function getFloat64Memory0() {
    if (cachedFloat64Memory0 === null || cachedFloat64Memory0.byteLength === 0) {
        cachedFloat64Memory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64Memory0;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => {
    wasm.__wbindgen_export_2.get(state.dtor)(state.a, state.b);
});

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_2.get(state.dtor)(a, state.b);
                CLOSURE_DTORS.unregister(state);
            } else {
                state.a = a;
            }
        }
    };
    real.original = state;
    CLOSURE_DTORS.register(real, state, state);
    return real;
}
function __wbg_adapter_40(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h2ea6f09f4b9fb508(arg0, arg1, addHeapObject(arg2));
}

let cachedUint32Memory0 = null;

function getUint32Memory0() {
    if (cachedUint32Memory0 === null || cachedUint32Memory0.byteLength === 0) {
        cachedUint32Memory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32Memory0;
}

function passArrayJsValueToWasm0(array, malloc) {
    const ptr = malloc(array.length * 4, 4) >>> 0;
    const mem = getUint32Memory0();
    for (let i = 0; i < array.length; i++) {
        mem[ptr / 4 + i] = addHeapObject(array[i]);
    }
    WASM_VECTOR_LEN = array.length;
    return ptr;
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8Memory0().subarray(ptr / 1, ptr / 1 + len);
}

function getArrayJsValueFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    const mem = getUint32Memory0();
    const slice = mem.subarray(ptr / 4, ptr / 4 + len);
    const result = [];
    for (let i = 0; i < slice.length; i++) {
        result.push(takeObject(slice[i]));
    }
    return result;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_exn_store(addHeapObject(e));
    }
}
function __wbg_adapter_222(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures__invoke2_mut__h6a857108eab6c9e3(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

const IntoUnderlyingByteSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingbytesource_free(ptr >>> 0));
/**
*/
class IntoUnderlyingByteSource {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingByteSourceFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingbytesource_free(ptr);
    }
    /**
    * @returns {string}
    */
    get type() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.intounderlyingbytesource_type(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
    * @returns {number}
    */
    get autoAllocateChunkSize() {
        const ret = wasm.intounderlyingbytesource_autoAllocateChunkSize(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
    * @param {ReadableByteStreamController} controller
    */
    start(controller) {
        wasm.intounderlyingbytesource_start(this.__wbg_ptr, addHeapObject(controller));
    }
    /**
    * @param {ReadableByteStreamController} controller
    * @returns {Promise<any>}
    */
    pull(controller) {
        const ret = wasm.intounderlyingbytesource_pull(this.__wbg_ptr, addHeapObject(controller));
        return takeObject(ret);
    }
    /**
    */
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingbytesource_cancel(ptr);
    }
}

const IntoUnderlyingSinkFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsink_free(ptr >>> 0));
/**
*/
class IntoUnderlyingSink {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSinkFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsink_free(ptr);
    }
    /**
    * @param {any} chunk
    * @returns {Promise<any>}
    */
    write(chunk) {
        const ret = wasm.intounderlyingsink_write(this.__wbg_ptr, addHeapObject(chunk));
        return takeObject(ret);
    }
    /**
    * @returns {Promise<any>}
    */
    close() {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_close(ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} reason
    * @returns {Promise<any>}
    */
    abort(reason) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_abort(ptr, addHeapObject(reason));
        return takeObject(ret);
    }
}

const IntoUnderlyingSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsource_free(ptr >>> 0));
/**
*/
class IntoUnderlyingSource {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSourceFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsource_free(ptr);
    }
    /**
    * @param {ReadableStreamDefaultController} controller
    * @returns {Promise<any>}
    */
    pull(controller) {
        const ret = wasm.intounderlyingsource_pull(this.__wbg_ptr, addHeapObject(controller));
        return takeObject(ret);
    }
    /**
    */
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingsource_cancel(ptr);
    }
}

const NewSwapTransactionResultFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_newswaptransactionresult_free(ptr >>> 0));
/**
*/
class NewSwapTransactionResult {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(NewSwapTransactionResult.prototype);
        obj.__wbg_ptr = ptr;
        NewSwapTransactionResultFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        NewSwapTransactionResultFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_newswaptransactionresult_free(ptr);
    }
    /**
    * @param {string} transaction_id
    * @param {(string)[]} expected_output_note_ids
    * @param {(string)[]} expected_partial_note_ids
    * @param {string | undefined} [payback_note_tag]
    * @returns {NewSwapTransactionResult}
    */
    static new(transaction_id, expected_output_note_ids, expected_partial_note_ids, payback_note_tag) {
        const ptr0 = passStringToWasm0(transaction_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayJsValueToWasm0(expected_output_note_ids, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passArrayJsValueToWasm0(expected_partial_note_ids, wasm.__wbindgen_malloc);
        const len2 = WASM_VECTOR_LEN;
        var ptr3 = isLikeNone(payback_note_tag) ? 0 : passStringToWasm0(payback_note_tag, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len3 = WASM_VECTOR_LEN;
        const ret = wasm.newswaptransactionresult_new(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3);
        return NewSwapTransactionResult.__wrap(ret);
    }
    /**
    * @param {string} payback_note_tag
    */
    set_note_tag(payback_note_tag) {
        const ptr0 = passStringToWasm0(payback_note_tag, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.newswaptransactionresult_set_note_tag(this.__wbg_ptr, ptr0, len0);
    }
    /**
    * @returns {string}
    */
    get transaction_id() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.newswaptransactionresult_transaction_id(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
    * @returns {any}
    */
    get expected_output_note_ids() {
        const ret = wasm.newswaptransactionresult_expected_output_note_ids(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    get expected_partial_note_ids() {
        const ret = wasm.newswaptransactionresult_expected_partial_note_ids(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
    * @returns {string}
    */
    get payback_note_tag() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.newswaptransactionresult_payback_note_tag(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}

const NewTransactionResultFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_newtransactionresult_free(ptr >>> 0));
/**
*/
class NewTransactionResult {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(NewTransactionResult.prototype);
        obj.__wbg_ptr = ptr;
        NewTransactionResultFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        NewTransactionResultFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_newtransactionresult_free(ptr);
    }
    /**
    * @param {string} transaction_id
    * @param {(string)[]} created_note_ids
    * @returns {NewTransactionResult}
    */
    static new(transaction_id, created_note_ids) {
        const ptr0 = passStringToWasm0(transaction_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayJsValueToWasm0(created_note_ids, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.newtransactionresult_new(ptr0, len0, ptr1, len1);
        return NewTransactionResult.__wrap(ret);
    }
    /**
    * @returns {string}
    */
    get transaction_id() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.newswaptransactionresult_transaction_id(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
    * @returns {any}
    */
    get created_note_ids() {
        const ret = wasm.newswaptransactionresult_expected_output_note_ids(this.__wbg_ptr);
        return takeObject(ret);
    }
}

const SerializedAccountStubFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_serializedaccountstub_free(ptr >>> 0));
/**
*/
class SerializedAccountStub {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(SerializedAccountStub.prototype);
        obj.__wbg_ptr = ptr;
        SerializedAccountStubFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        SerializedAccountStubFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_serializedaccountstub_free(ptr);
    }
    /**
    * @param {string} id
    * @param {string} nonce
    * @param {string} vault_root
    * @param {string} storage_root
    * @param {string} code_root
    * @returns {SerializedAccountStub}
    */
    static new(id, nonce, vault_root, storage_root, code_root) {
        const ptr0 = passStringToWasm0(id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(nonce, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(vault_root, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(storage_root, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(code_root, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ret = wasm.serializedaccountstub_new(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4);
        return SerializedAccountStub.__wrap(ret);
    }
    /**
    * @returns {string}
    */
    get id() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.newswaptransactionresult_transaction_id(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
    * @returns {string}
    */
    get nonce() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.serializedaccountstub_nonce(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
    * @returns {string}
    */
    get vault_root() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.serializedaccountstub_vault_root(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
    * @returns {string}
    */
    get storage_root() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.newswaptransactionresult_payback_note_tag(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
    * @returns {string}
    */
    get code_root() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.serializedaccountstub_code_root(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}

const WebClientFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_webclient_free(ptr >>> 0));
/**
*/
let WebClient$1 = class WebClient {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WebClientFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_webclient_free(ptr);
    }
    /**
    * @returns {Promise<any>}
    */
    get_accounts() {
        const ret = wasm.webclient_get_accounts(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
    * @param {string} account_id
    * @returns {Promise<any>}
    */
    get_account(account_id) {
        const ptr0 = passStringToWasm0(account_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.webclient_get_account(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * @param {any} pub_key_bytes
    * @returns {any}
    */
    get_account_auth_by_pub_key(pub_key_bytes) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.webclient_get_account_auth_by_pub_key(retptr, this.__wbg_ptr, addHeapObject(pub_key_bytes));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {string} account_id
    * @returns {Promise<any>}
    */
    fetch_and_cache_account_auth_by_pub_key(account_id) {
        const ptr0 = passStringToWasm0(account_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.webclient_fetch_and_cache_account_auth_by_pub_key(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * @param {string} note_id
    * @returns {Promise<any>}
    */
    export_note(note_id) {
        const ptr0 = passStringToWasm0(note_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.webclient_export_note(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * @param {any} account_bytes
    * @returns {Promise<any>}
    */
    import_account(account_bytes) {
        const ret = wasm.webclient_import_account(this.__wbg_ptr, addHeapObject(account_bytes));
        return takeObject(ret);
    }
    /**
    * @param {any} note_bytes
    * @param {boolean} verify
    * @returns {Promise<any>}
    */
    import_note(note_bytes, verify) {
        const ret = wasm.webclient_import_note(this.__wbg_ptr, addHeapObject(note_bytes), verify);
        return takeObject(ret);
    }
    /**
    * @param {string} storage_type
    * @param {boolean} mutable
    * @returns {Promise<any>}
    */
    new_wallet(storage_type, mutable) {
        const ptr0 = passStringToWasm0(storage_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.webclient_new_wallet(this.__wbg_ptr, ptr0, len0, mutable);
        return takeObject(ret);
    }
    /**
    * @param {string} storage_type
    * @param {boolean} non_fungible
    * @param {string} token_symbol
    * @param {string} decimals
    * @param {string} max_supply
    * @returns {Promise<any>}
    */
    new_faucet(storage_type, non_fungible, token_symbol, decimals, max_supply) {
        const ptr0 = passStringToWasm0(storage_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(token_symbol, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(decimals, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(max_supply, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ret = wasm.webclient_new_faucet(this.__wbg_ptr, ptr0, len0, non_fungible, ptr1, len1, ptr2, len2, ptr3, len3);
        return takeObject(ret);
    }
    /**
    * @param {string} target_account_id
    * @param {string} faucet_id
    * @param {string} note_type
    * @param {string} amount
    * @returns {Promise<NewTransactionResult>}
    */
    new_mint_transaction(target_account_id, faucet_id, note_type, amount) {
        const ptr0 = passStringToWasm0(target_account_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(faucet_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(note_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ret = wasm.webclient_new_mint_transaction(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3);
        return takeObject(ret);
    }
    /**
    * @param {string} sender_account_id
    * @param {string} target_account_id
    * @param {string} faucet_id
    * @param {string} note_type
    * @param {string} amount
    * @param {string | undefined} [recall_height]
    * @returns {Promise<NewTransactionResult>}
    */
    new_send_transaction(sender_account_id, target_account_id, faucet_id, note_type, amount, recall_height) {
        const ptr0 = passStringToWasm0(sender_account_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(target_account_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(faucet_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(note_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        var ptr5 = isLikeNone(recall_height) ? 0 : passStringToWasm0(recall_height, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len5 = WASM_VECTOR_LEN;
        const ret = wasm.webclient_new_send_transaction(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5);
        return takeObject(ret);
    }
    /**
    * @param {string} account_id
    * @param {(string)[]} list_of_notes
    * @returns {Promise<NewTransactionResult>}
    */
    new_consume_transaction(account_id, list_of_notes) {
        const ptr0 = passStringToWasm0(account_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayJsValueToWasm0(list_of_notes, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.webclient_new_consume_transaction(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return takeObject(ret);
    }
    /**
    * @param {string} sender_account_id
    * @param {string} offered_asset_faucet_id
    * @param {string} offered_asset_amount
    * @param {string} requested_asset_faucet_id
    * @param {string} requested_asset_amount
    * @param {string} note_type
    * @returns {Promise<NewSwapTransactionResult>}
    */
    new_swap_transaction(sender_account_id, offered_asset_faucet_id, offered_asset_amount, requested_asset_faucet_id, requested_asset_amount, note_type) {
        const ptr0 = passStringToWasm0(sender_account_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(offered_asset_faucet_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(offered_asset_amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(requested_asset_faucet_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(requested_asset_amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ptr5 = passStringToWasm0(note_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len5 = WASM_VECTOR_LEN;
        const ret = wasm.webclient_new_swap_transaction(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5);
        return takeObject(ret);
    }
    /**
    * @param {any} filter
    * @returns {Promise<any>}
    */
    get_input_notes(filter) {
        const ret = wasm.webclient_get_input_notes(this.__wbg_ptr, addHeapObject(filter));
        return takeObject(ret);
    }
    /**
    * @param {string} note_id
    * @returns {Promise<any>}
    */
    get_input_note(note_id) {
        const ptr0 = passStringToWasm0(note_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.webclient_get_input_note(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * @param {any} filter
    * @returns {Promise<any>}
    */
    get_output_notes(filter) {
        const ret = wasm.webclient_get_output_notes(this.__wbg_ptr, addHeapObject(filter));
        return takeObject(ret);
    }
    /**
    * @param {string} note_id
    * @returns {Promise<any>}
    */
    get_output_note(note_id) {
        const ptr0 = passStringToWasm0(note_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.webclient_get_output_note(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * @returns {Promise<any>}
    */
    sync_state() {
        const ret = wasm.webclient_sync_state(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
    * @param {string} tag
    * @returns {Promise<any>}
    */
    add_tag(tag) {
        const ptr0 = passStringToWasm0(tag, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.webclient_add_tag(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * @returns {Promise<any>}
    */
    get_transactions() {
        const ret = wasm.webclient_get_transactions(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
    */
    constructor() {
        const ret = wasm.webclient_new();
        this.__wbg_ptr = ret >>> 0;
        return this;
    }
    /**
    * @param {string | undefined} [node_url]
    * @returns {Promise<any>}
    */
    create_client(node_url) {
        var ptr0 = isLikeNone(node_url) ? 0 : passStringToWasm0(node_url, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        const ret = wasm.webclient_create_client(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
};

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

function __wbg_get_imports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        const ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_isArray_2ab64d95e09ea0ae = function(arg0) {
        const ret = Array.isArray(getObject(arg0));
        return ret;
    };
    imports.wbg.__wbg_length_cd7af8117672b8b8 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_get_bd8e338fbd5f5cc8 = function(arg0, arg1) {
        const ret = getObject(arg0)[arg1 >>> 0];
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'string' ? obj : undefined;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbg_next_196c84450b364254 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).next();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_done_298b57d23c0fc80c = function(arg0) {
        const ret = getObject(arg0).done;
        return ret;
    };
    imports.wbg.__wbg_value_d93c65011f51a456 = function(arg0) {
        const ret = getObject(arg0).value;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_is_string = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'string';
        return ret;
    };
    imports.wbg.__wbindgen_is_object = function(arg0) {
        const val = getObject(arg0);
        const ret = typeof(val) === 'object' && val !== null;
        return ret;
    };
    imports.wbg.__wbg_entries_95cc2c823b285a09 = function(arg0) {
        const ret = Object.entries(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newswaptransactionresult_new = function(arg0) {
        const ret = NewSwapTransactionResult.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_call_b3ca7c6051f9bec1 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_new_16b304a2cfa7ff4a = function() {
        const ret = new Array();
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_number_new = function(arg0) {
        const ret = arg0;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_set_d4638f722068f043 = function(arg0, arg1, arg2) {
        getObject(arg0)[arg1 >>> 0] = takeObject(arg2);
    };
    imports.wbg.__wbg_new_72fb9a18b5ae2624 = function() {
        const ret = new Object();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_set_f975102236d3c502 = function(arg0, arg1, arg2) {
        getObject(arg0)[takeObject(arg1)] = takeObject(arg2);
    };
    imports.wbg.__wbg_newtransactionresult_new = function(arg0) {
        const ret = NewTransactionResult.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_crypto_1d1f22824a6a080c = function(arg0) {
        const ret = getObject(arg0).crypto;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_process_4a72847cc503995b = function(arg0) {
        const ret = getObject(arg0).process;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_versions_f686565e586dd935 = function(arg0) {
        const ret = getObject(arg0).versions;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_node_104a2ff8d6ea03a2 = function(arg0) {
        const ret = getObject(arg0).node;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_require_cca90b1a94a0255b = function() { return handleError(function () {
        const ret = module.require;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_is_function = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'function';
        return ret;
    };
    imports.wbg.__wbg_msCrypto_eb05e62b530a1508 = function(arg0) {
        const ret = getObject(arg0).msCrypto;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithlength_e9b4878cebadb3d3 = function(arg0) {
        const ret = new Uint8Array(arg0 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_memory = function() {
        const ret = wasm.memory;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_buffer_12d079cc21e14bdb = function(arg0) {
        const ret = getObject(arg0).buffer;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_aa4a17c33a06e5cb = function(arg0, arg1, arg2) {
        const ret = new Uint8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_randomFillSync_5c9c955aa56b6049 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).randomFillSync(takeObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_subarray_a1f73cd4b5b42fe1 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).subarray(arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getRandomValues_3aa56aa6edec874c = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).getRandomValues(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_new_63b92bc8671ed464 = function(arg0) {
        const ret = new Uint8Array(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_set_a47bac70306a19a7 = function(arg0, arg1, arg2) {
        getObject(arg0).set(getObject(arg1), arg2 >>> 0);
    };
    imports.wbg.__wbg_openDatabase_480758c2af4b6033 = function() {
        const ret = openDatabase();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_fetchAndCacheAccountAuthByPubKey_6394158327160df7 = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            const ret = fetchAndCacheAccountAuthByPubKey(getStringFromWasm0(arg0, arg1));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_log_5bb5f88f245d7762 = function(arg0) {
        console.log(getObject(arg0));
    };
    imports.wbg.__wbg_addNoteTag_18498f60196ea607 = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            const ret = addNoteTag(getStringFromWasm0(arg0, arg1));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbindgen_cb_drop = function(arg0) {
        const obj = takeObject(arg0).original;
        if (obj.cnt-- == 1) {
            obj.a = 0;
            return true;
        }
        const ret = false;
        return ret;
    };
    imports.wbg.__wbindgen_error_new = function(arg0, arg1) {
        const ret = new Error(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_isSafeInteger_f7b04ef02296c4d2 = function(arg0) {
        const ret = Number.isSafeInteger(getObject(arg0));
        return ret;
    };
    imports.wbg.__wbindgen_as_number = function(arg0) {
        const ret = +getObject(arg0);
        return ret;
    };
    imports.wbg.__wbg_getwithrefkey_edc2c8960f0f1191 = function(arg0, arg1) {
        const ret = getObject(arg0)[getObject(arg1)];
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        const ret = getObject(arg0) === undefined;
        return ret;
    };
    imports.wbg.__wbindgen_in = function(arg0, arg1) {
        const ret = getObject(arg0) in getObject(arg1);
        return ret;
    };
    imports.wbg.__wbindgen_jsval_loose_eq = function(arg0, arg1) {
        const ret = getObject(arg0) == getObject(arg1);
        return ret;
    };
    imports.wbg.__wbindgen_boolean_get = function(arg0) {
        const v = getObject(arg0);
        const ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
        return ret;
    };
    imports.wbg.__wbg_applyStateSync_c8b7a83a68ef087d = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11, arg12, arg13, arg14, arg15, arg16, arg17, arg18, arg19, arg20, arg21, arg22, arg23, arg24, arg25, arg26, arg27, arg28) {
        let deferred0_0;
        let deferred0_1;
        let deferred3_0;
        let deferred3_1;
        let deferred4_0;
        let deferred4_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            var v1 = getArrayJsValueFromWasm0(arg2, arg3).slice();
            wasm.__wbindgen_free(arg2, arg3 * 4, 4);
            var v2 = getArrayJsValueFromWasm0(arg4, arg5).slice();
            wasm.__wbindgen_free(arg4, arg5 * 4, 4);
            deferred3_0 = arg6;
            deferred3_1 = arg7;
            deferred4_0 = arg8;
            deferred4_1 = arg9;
            var v5 = getArrayJsValueFromWasm0(arg11, arg12).slice();
            wasm.__wbindgen_free(arg11, arg12 * 4, 4);
            var v6 = getArrayJsValueFromWasm0(arg13, arg14).slice();
            wasm.__wbindgen_free(arg13, arg14 * 4, 4);
            var v7 = getArrayJsValueFromWasm0(arg15, arg16).slice();
            wasm.__wbindgen_free(arg15, arg16 * 4, 4);
            var v8 = getArrayJsValueFromWasm0(arg17, arg18).slice();
            wasm.__wbindgen_free(arg17, arg18 * 4, 4);
            var v9 = getArrayJsValueFromWasm0(arg19, arg20).slice();
            wasm.__wbindgen_free(arg19, arg20 * 4, 4);
            var v10 = getArrayJsValueFromWasm0(arg21, arg22).slice();
            wasm.__wbindgen_free(arg21, arg22 * 4, 4);
            var v11 = getArrayJsValueFromWasm0(arg23, arg24).slice();
            wasm.__wbindgen_free(arg23, arg24 * 4, 4);
            var v12 = getArrayJsValueFromWasm0(arg25, arg26).slice();
            wasm.__wbindgen_free(arg25, arg26 * 4, 4);
            var v13 = getArrayJsValueFromWasm0(arg27, arg28).slice();
            wasm.__wbindgen_free(arg27, arg28 * 4, 4);
            const ret = applyStateSync(getStringFromWasm0(arg0, arg1), v1, v2, getStringFromWasm0(arg6, arg7), getStringFromWasm0(arg8, arg9), arg10 !== 0, v5, v6, v7, v8, v9, v10, v11, v12, v13);
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    };
    imports.wbg.__wbg_insertAccountStorage_991970c252435210 = function(arg0, arg1, arg2, arg3) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            var v1 = getArrayU8FromWasm0(arg2, arg3).slice();
            wasm.__wbindgen_free(arg2, arg3 * 1, 1);
            const ret = insertAccountStorage(getStringFromWasm0(arg0, arg1), v1);
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_insertAccountAssetVault_2707f8b930a79a6f = function(arg0, arg1, arg2, arg3) {
        let deferred0_0;
        let deferred0_1;
        let deferred1_0;
        let deferred1_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            deferred1_0 = arg2;
            deferred1_1 = arg3;
            const ret = insertAccountAssetVault(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    };
    imports.wbg.__wbg_insertAccountRecord_0036bcd5c8c26dcc = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11, arg12) {
        let deferred0_0;
        let deferred0_1;
        let deferred1_0;
        let deferred1_1;
        let deferred2_0;
        let deferred2_1;
        let deferred3_0;
        let deferred3_1;
        let deferred4_0;
        let deferred4_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            deferred1_0 = arg2;
            deferred1_1 = arg3;
            deferred2_0 = arg4;
            deferred2_1 = arg5;
            deferred3_0 = arg6;
            deferred3_1 = arg7;
            deferred4_0 = arg8;
            deferred4_1 = arg9;
            let v5;
            if (arg11 !== 0) {
                v5 = getArrayU8FromWasm0(arg11, arg12).slice();
                wasm.__wbindgen_free(arg11, arg12 * 1, 1);
            }
            const ret = insertAccountRecord(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3), getStringFromWasm0(arg4, arg5), getStringFromWasm0(arg6, arg7), getStringFromWasm0(arg8, arg9), arg10 !== 0, v5);
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    };
    imports.wbg.__wbg_getAccountStub_24cc01b27d682ba5 = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            const ret = getAccountStub(getStringFromWasm0(arg0, arg1));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_getAccountCode_32ce8567df6abdd4 = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            const ret = getAccountCode(getStringFromWasm0(arg0, arg1));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_getAccountStorage_88952c10a636e0cc = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            const ret = getAccountStorage(getStringFromWasm0(arg0, arg1));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_getAccountAssetVault_717c56f9a7ce78a6 = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            const ret = getAccountAssetVault(getStringFromWasm0(arg0, arg1));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_getAccountAuthByPubKey_e5418393b65bda96 = function(arg0, arg1) {
        var v0 = getArrayU8FromWasm0(arg0, arg1).slice();
        wasm.__wbindgen_free(arg0, arg1 * 1, 1);
        const ret = getAccountAuthByPubKey(v0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_insertInputNote_6d3903d2cef50e14 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11, arg12, arg13, arg14, arg15, arg16, arg17, arg18, arg19) {
        let deferred0_0;
        let deferred0_1;
        let deferred2_0;
        let deferred2_1;
        let deferred3_0;
        let deferred3_1;
        let deferred5_0;
        let deferred5_1;
        let deferred6_0;
        let deferred6_1;
        let deferred9_0;
        let deferred9_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            var v1 = getArrayU8FromWasm0(arg2, arg3).slice();
            wasm.__wbindgen_free(arg2, arg3 * 1, 1);
            deferred2_0 = arg4;
            deferred2_1 = arg5;
            deferred3_0 = arg6;
            deferred3_1 = arg7;
            let v4;
            if (arg8 !== 0) {
                v4 = getStringFromWasm0(arg8, arg9).slice();
                wasm.__wbindgen_free(arg8, arg9 * 1, 1);
            }
            deferred5_0 = arg10;
            deferred5_1 = arg11;
            deferred6_0 = arg12;
            deferred6_1 = arg13;
            var v7 = getArrayU8FromWasm0(arg14, arg15).slice();
            wasm.__wbindgen_free(arg14, arg15 * 1, 1);
            let v8;
            if (arg16 !== 0) {
                v8 = getStringFromWasm0(arg16, arg17).slice();
                wasm.__wbindgen_free(arg16, arg17 * 1, 1);
            }
            deferred9_0 = arg18;
            deferred9_1 = arg19;
            const ret = insertInputNote(getStringFromWasm0(arg0, arg1), v1, getStringFromWasm0(arg4, arg5), getStringFromWasm0(arg6, arg7), v4, getStringFromWasm0(arg10, arg11), getStringFromWasm0(arg12, arg13), v7, v8, getStringFromWasm0(arg18, arg19));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
            wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
            wasm.__wbindgen_free(deferred6_0, deferred6_1, 1);
            wasm.__wbindgen_free(deferred9_0, deferred9_1, 1);
        }
    };
    imports.wbg.__wbg_getNoteTags_475d8f3b44d2c8d9 = function() {
        const ret = getNoteTags();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_insertTransactionScript_440d0803ed20cbd6 = function(arg0, arg1, arg2, arg3) {
        var v0 = getArrayU8FromWasm0(arg0, arg1).slice();
        wasm.__wbindgen_free(arg0, arg1 * 1, 1);
        let v1;
        if (arg2 !== 0) {
            v1 = getArrayU8FromWasm0(arg2, arg3).slice();
            wasm.__wbindgen_free(arg2, arg3 * 1, 1);
        }
        const ret = insertTransactionScript(v0, v1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_insertProvenTransactionData_51e4f309584027c7 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11, arg12, arg13, arg14, arg15, arg16, arg17, arg18, arg19) {
        let deferred0_0;
        let deferred0_1;
        let deferred1_0;
        let deferred1_1;
        let deferred2_0;
        let deferred2_1;
        let deferred3_0;
        let deferred3_1;
        let deferred4_0;
        let deferred4_1;
        let deferred8_0;
        let deferred8_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            deferred1_0 = arg2;
            deferred1_1 = arg3;
            deferred2_0 = arg4;
            deferred2_1 = arg5;
            deferred3_0 = arg6;
            deferred3_1 = arg7;
            deferred4_0 = arg8;
            deferred4_1 = arg9;
            var v5 = getArrayU8FromWasm0(arg10, arg11).slice();
            wasm.__wbindgen_free(arg10, arg11 * 1, 1);
            let v6;
            if (arg12 !== 0) {
                v6 = getArrayU8FromWasm0(arg12, arg13).slice();
                wasm.__wbindgen_free(arg12, arg13 * 1, 1);
            }
            let v7;
            if (arg14 !== 0) {
                v7 = getStringFromWasm0(arg14, arg15).slice();
                wasm.__wbindgen_free(arg14, arg15 * 1, 1);
            }
            deferred8_0 = arg16;
            deferred8_1 = arg17;
            let v9;
            if (arg18 !== 0) {
                v9 = getStringFromWasm0(arg18, arg19).slice();
                wasm.__wbindgen_free(arg18, arg19 * 1, 1);
            }
            const ret = insertProvenTransactionData(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3), getStringFromWasm0(arg4, arg5), getStringFromWasm0(arg6, arg7), getStringFromWasm0(arg8, arg9), v5, v6, v7, getStringFromWasm0(arg16, arg17), v9);
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
            wasm.__wbindgen_free(deferred8_0, deferred8_1, 1);
        }
    };
    imports.wbg.__wbg_insertOutputNote_39ac93ea0dbc9249 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11, arg12, arg13, arg14, arg15, arg16, arg17, arg18, arg19) {
        let deferred0_0;
        let deferred0_1;
        let deferred2_0;
        let deferred2_1;
        let deferred3_0;
        let deferred3_1;
        let deferred4_0;
        let deferred4_1;
        let deferred9_0;
        let deferred9_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            var v1 = getArrayU8FromWasm0(arg2, arg3).slice();
            wasm.__wbindgen_free(arg2, arg3 * 1, 1);
            deferred2_0 = arg4;
            deferred2_1 = arg5;
            deferred3_0 = arg6;
            deferred3_1 = arg7;
            deferred4_0 = arg8;
            deferred4_1 = arg9;
            let v5;
            if (arg10 !== 0) {
                v5 = getStringFromWasm0(arg10, arg11).slice();
                wasm.__wbindgen_free(arg10, arg11 * 1, 1);
            }
            let v6;
            if (arg12 !== 0) {
                v6 = getStringFromWasm0(arg12, arg13).slice();
                wasm.__wbindgen_free(arg12, arg13 * 1, 1);
            }
            let v7;
            if (arg14 !== 0) {
                v7 = getArrayU8FromWasm0(arg14, arg15).slice();
                wasm.__wbindgen_free(arg14, arg15 * 1, 1);
            }
            let v8;
            if (arg16 !== 0) {
                v8 = getStringFromWasm0(arg16, arg17).slice();
                wasm.__wbindgen_free(arg16, arg17 * 1, 1);
            }
            deferred9_0 = arg18;
            deferred9_1 = arg19;
            const ret = insertOutputNote(getStringFromWasm0(arg0, arg1), v1, getStringFromWasm0(arg4, arg5), getStringFromWasm0(arg6, arg7), getStringFromWasm0(arg8, arg9), v5, v6, v7, v8, getStringFromWasm0(arg18, arg19));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
            wasm.__wbindgen_free(deferred9_0, deferred9_1, 1);
        }
    };
    imports.wbg.__wbg_updateNoteConsumerTxId_46b25df46a766b2d = function(arg0, arg1, arg2, arg3, arg4, arg5) {
        let deferred0_0;
        let deferred0_1;
        let deferred1_0;
        let deferred1_1;
        let deferred2_0;
        let deferred2_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            deferred1_0 = arg2;
            deferred1_1 = arg3;
            deferred2_0 = arg4;
            deferred2_1 = arg5;
            const ret = updateNoteConsumerTxId(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3), getStringFromWasm0(arg4, arg5));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    };
    imports.wbg.__wbg_insertAccountCode_6a50fd9c8262245b = function(arg0, arg1, arg2, arg3, arg4, arg5) {
        let deferred0_0;
        let deferred0_1;
        let deferred1_0;
        let deferred1_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            deferred1_0 = arg2;
            deferred1_1 = arg3;
            var v2 = getArrayU8FromWasm0(arg4, arg5).slice();
            wasm.__wbindgen_free(arg4, arg5 * 1, 1);
            const ret = insertAccountCode(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3), v2);
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    };
    imports.wbg.__wbg_insertAccountAuth_c377aafc7378a406 = function(arg0, arg1, arg2, arg3, arg4, arg5) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            var v1 = getArrayU8FromWasm0(arg2, arg3).slice();
            wasm.__wbindgen_free(arg2, arg3 * 1, 1);
            var v2 = getArrayU8FromWasm0(arg4, arg5).slice();
            wasm.__wbindgen_free(arg4, arg5 * 1, 1);
            const ret = insertAccountAuth(getStringFromWasm0(arg0, arg1), v1, v2);
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_getAccountIds_d4d7acc998fd1306 = function() {
        const ret = getAccountIds();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getAccountAuth_ebf0292dbbff4efa = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            const ret = getAccountAuth(getStringFromWasm0(arg0, arg1));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_new_81740750da40724f = function(arg0, arg1) {
        try {
            var state0 = {a: arg0, b: arg1};
            var cb0 = (arg0, arg1) => {
                const a = state0.a;
                state0.a = 0;
                try {
                    return __wbg_adapter_222(a, state0.b, arg0, arg1);
                } finally {
                    state0.a = a;
                }
            };
            const ret = new Promise(cb0);
            return addHeapObject(ret);
        } finally {
            state0.a = state0.b = 0;
        }
    };
    imports.wbg.__wbg_insertChainMmrNodes_10b43c8fd72a653a = function(arg0, arg1, arg2, arg3) {
        var v0 = getArrayJsValueFromWasm0(arg0, arg1).slice();
        wasm.__wbindgen_free(arg0, arg1 * 4, 4);
        var v1 = getArrayJsValueFromWasm0(arg2, arg3).slice();
        wasm.__wbindgen_free(arg2, arg3 * 4, 4);
        const ret = insertChainMmrNodes(v0, v1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getSyncHeight_6cfec9656fbe7a54 = function() {
        const ret = getSyncHeight();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getTransactions_615710f87b3d7a68 = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            const ret = getTransactions(getStringFromWasm0(arg0, arg1));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_getInputNotesFromIds_8d51f47927f23f1e = function(arg0, arg1) {
        var v0 = getArrayJsValueFromWasm0(arg0, arg1).slice();
        wasm.__wbindgen_free(arg0, arg1 * 4, 4);
        const ret = getInputNotesFromIds(v0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getInputNotes_cffb103a0c156802 = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            const ret = getInputNotes(getStringFromWasm0(arg0, arg1));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_getOutputNotesFromIds_94bc090d40b285c8 = function(arg0, arg1) {
        var v0 = getArrayJsValueFromWasm0(arg0, arg1).slice();
        wasm.__wbindgen_free(arg0, arg1 * 4, 4);
        const ret = getOutputNotesFromIds(v0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getOutputNotes_805dfd1a77a9bd62 = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            const ret = getOutputNotes(getStringFromWasm0(arg0, arg1));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_insertBlockHeader_6cc1c8938526002a = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
        let deferred0_0;
        let deferred0_1;
        let deferred1_0;
        let deferred1_1;
        let deferred2_0;
        let deferred2_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            deferred1_0 = arg2;
            deferred1_1 = arg3;
            deferred2_0 = arg4;
            deferred2_1 = arg5;
            const ret = insertBlockHeader(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3), getStringFromWasm0(arg4, arg5), arg6 !== 0);
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    };
    imports.wbg.__wbg_getBlockHeaders_f6f2f7652cfd53a2 = function(arg0, arg1) {
        var v0 = getArrayJsValueFromWasm0(arg0, arg1).slice();
        wasm.__wbindgen_free(arg0, arg1 * 4, 4);
        const ret = getBlockHeaders(v0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getChainMmrNodesAll_9b6885bedde9cd16 = function() {
        const ret = getChainMmrNodesAll();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getChainMmrNodes_73e7f003e2d2937d = function(arg0, arg1) {
        var v0 = getArrayJsValueFromWasm0(arg0, arg1).slice();
        wasm.__wbindgen_free(arg0, arg1 * 4, 4);
        const ret = getChainMmrNodes(v0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getChainMmrPeaksByBlockNum_986a22f1befb4775 = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            const ret = getChainMmrPeaksByBlockNum(getStringFromWasm0(arg0, arg1));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_getAllAccountStubs_4b6c7212d0f4ff29 = function() {
        const ret = getAllAccountStubs();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getUnspentInputNoteNullifiers_927c58ec7d29b1e9 = function() {
        const ret = getUnspentInputNoteNullifiers();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new0_7d84e5b2cd9fdc73 = function() {
        const ret = new Date();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getTime_2bc4375165f02d15 = function(arg0) {
        const ret = getObject(arg0).getTime();
        return ret;
    };
    imports.wbg.__wbg_iterator_2cee6dadfd956dfa = function() {
        const ret = Symbol.iterator;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_get_e3c254076557e348 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.get(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_call_27c0f87801dedf93 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).call(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_next_40fc327bfc8770e6 = function(arg0) {
        const ret = getObject(arg0).next;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_self_ce0dbfc45cf2f5be = function() { return handleError(function () {
        const ret = self.self;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_window_c6fb939a7f436783 = function() { return handleError(function () {
        const ret = window.window;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_globalThis_d1e6af4856ba331b = function() { return handleError(function () {
        const ret = globalThis.globalThis;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_global_207b558942527489 = function() { return handleError(function () {
        const ret = global.global;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newnoargs_e258087cd0daa0ea = function(arg0, arg1) {
        const ret = new Function(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
        const ret = getObject(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_length_c20a40f15020d68a = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'number' ? obj : undefined;
        getFloat64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? 0 : ret;
        getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
    };
    imports.wbg.__wbg_instanceof_Uint8Array_2b3bbecd033d19f6 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Uint8Array;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_ArrayBuffer_836825be07d4c9d2 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof ArrayBuffer;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_toString_c816a20ab859d0c1 = function(arg0) {
        const ret = getObject(arg0).toString();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_ab6fd82b10560829 = function() { return handleError(function () {
        const ret = new Headers();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_append_7bfcb4937d1d5e29 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).append(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_set_1f9b04f170055d33 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_newwithstrandinit_3fd6fba4083ff2d0 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = new Request(getStringFromWasm0(arg0, arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_has_0af94d20077affa2 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.has(getObject(arg0), getObject(arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_fetch_40cabeda000226f7 = function(arg0, arg1) {
        const ret = fetch(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_fetch_bc400efeda8ac0c8 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).fetch(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_status_61a01141acd3cf74 = function(arg0) {
        const ret = getObject(arg0).status;
        return ret;
    };
    imports.wbg.__wbg_headers_9620bfada380764a = function(arg0) {
        const ret = getObject(arg0).headers;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_body_9545a94f397829db = function(arg0) {
        const ret = getObject(arg0).body;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_getReader_ab94afcb5cb7689a = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).getReader();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_releaseLock_5c49db976c08b864 = function(arg0) {
        getObject(arg0).releaseLock();
    };
    imports.wbg.__wbg_read_e7d0f8a49be01d86 = function(arg0) {
        const ret = getObject(arg0).read();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_done_2ffa852272310e47 = function(arg0) {
        const ret = getObject(arg0).done;
        return ret;
    };
    imports.wbg.__wbg_value_9f6eeb1e2aab8d96 = function(arg0) {
        const ret = getObject(arg0).value;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
        const ret = debugString(getObject(arg1));
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbg_then_a73caa9a87991566 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_queueMicrotask_3cbae2ec6b6cd3d6 = function(arg0) {
        const ret = getObject(arg0).queueMicrotask;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_resolve_b0083a7967828ec8 = function(arg0) {
        const ret = Promise.resolve(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_then_0c86a60e8fcfe9f6 = function(arg0, arg1) {
        const ret = getObject(arg0).then(getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_queueMicrotask_481971b0d87f3dd4 = function(arg0) {
        queueMicrotask(getObject(arg0));
    };
    imports.wbg.__wbg_cancel_6ee33d4006737aef = function(arg0) {
        const ret = getObject(arg0).cancel();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_catch_0260e338d10f79ae = function(arg0, arg1) {
        const ret = getObject(arg0).catch(getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_byobRequest_72fca99f9c32c193 = function(arg0) {
        const ret = getObject(arg0).byobRequest;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_view_7f0ce470793a340f = function(arg0) {
        const ret = getObject(arg0).view;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_byteLength_58f7b4fab1919d44 = function(arg0) {
        const ret = getObject(arg0).byteLength;
        return ret;
    };
    imports.wbg.__wbg_close_184931724d961ccc = function() { return handleError(function (arg0) {
        getObject(arg0).close();
    }, arguments) };
    imports.wbg.__wbg_new_28c511d9baebfa89 = function(arg0, arg1) {
        const ret = new Error(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_buffer_dd7f74bc60f1faab = function(arg0) {
        const ret = getObject(arg0).buffer;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_byteOffset_81d60f7392524f62 = function(arg0) {
        const ret = getObject(arg0).byteOffset;
        return ret;
    };
    imports.wbg.__wbg_respond_b1a43b2e3a06d525 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).respond(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_close_a994f9425dab445c = function() { return handleError(function (arg0) {
        getObject(arg0).close();
    }, arguments) };
    imports.wbg.__wbg_enqueue_ea194723156c0cc2 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).enqueue(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbindgen_closure_wrapper3309 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 135, __wbg_adapter_40);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper3330 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 135, __wbg_adapter_40);
        return addHeapObject(ret);
    };

    return imports;
}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedFloat64Memory0 = null;
    cachedInt32Memory0 = null;
    cachedUint32Memory0 = null;
    cachedUint8Memory0 = null;


    return wasm;
}

function initSync(module) {
    if (wasm !== undefined) return wasm;

    const imports = __wbg_get_imports();

    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }

    const instance = new WebAssembly.Instance(module, imports);

    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(input) {
    if (wasm !== undefined) return wasm;


    const imports = __wbg_get_imports();

    if (typeof input === 'string' || (typeof Request === 'function' && input instanceof Request) || (typeof URL === 'function' && input instanceof URL)) {
        input = fetch(input);
    }

    const { instance, module } = await __wbg_load(await input, imports);

    return __wbg_finalize_init(instance, module);
}

var exports = /*#__PURE__*/Object.freeze({
  __proto__: null,
  IntoUnderlyingByteSource: IntoUnderlyingByteSource,
  IntoUnderlyingSink: IntoUnderlyingSink,
  IntoUnderlyingSource: IntoUnderlyingSource,
  NewSwapTransactionResult: NewSwapTransactionResult,
  NewTransactionResult: NewTransactionResult,
  SerializedAccountStub: SerializedAccountStub,
  WebClient: WebClient$1,
  default: __wbg_init,
  initSync: initSync
});

const wasm_path = "assets/miden_client.wasm";

            
            var Cargo = async (opt = {}) => {
                let {importHook, serverPath, initializeHook} = opt;

                let final_path = wasm_path;

                if (serverPath != null) {
                    final_path = serverPath + /[^\/\\]*$/.exec(final_path)[0];
                }

                if (importHook != null) {
                    final_path = importHook(final_path);
                }

                if (initializeHook != null) {
                    await initializeHook(__wbg_init, final_path);

                } else {
                    await __wbg_init(final_path);
                }

                return exports;
            };

const {
    WebClient
} = await Cargo({
    importHook: () => {
        return new URL("assets/miden_client.wasm", import.meta.url);
    },
});

export { WebClient };
//# sourceMappingURL=index.js.map
