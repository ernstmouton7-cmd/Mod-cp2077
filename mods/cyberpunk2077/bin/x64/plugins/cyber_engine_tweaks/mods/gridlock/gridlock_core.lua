-- Gridlock core v0.1.0 — generated from src/core by TypeScriptToLua. Do not edit by hand.
-- LuaJIT: the simulation has many branches; the default trace cache is too small
-- and thrashes (10x slowdown). Enlarge it when the jit module is available.
if jit and jit.opt and jit.opt.start then pcall(jit.opt.start, "maxtrace=8000", "maxmcode=16384") end
--[[ Generated with https://github.com/TypeScriptToLua/TypeScriptToLua ]]

local ____modules = {}
local ____moduleCache = {}
local ____originalRequire = require
local function require(file, ...)
    if ____moduleCache[file] then
        return ____moduleCache[file].value
    end
    if ____modules[file] then
        local module = ____modules[file]
        local value = nil
        if (select("#", ...) > 0) then value = module(...) else value = module(file) end
        ____moduleCache[file] = { value = value }
        return value
    else
        if ____originalRequire then
            return ____originalRequire(file)
        else
            error("module '" .. file .. "' not found")
        end
    end
end
____modules = {
["lualib_bundle"] = function(...) 
local function __TS__ArrayAt(self, relativeIndex)
    local absoluteIndex = relativeIndex < 0 and #self + relativeIndex or relativeIndex
    if absoluteIndex >= 0 and absoluteIndex < #self then
        return self[absoluteIndex + 1]
    end
    return nil
end

local function __TS__ArrayIsArray(value)
    return type(value) == "table" and (value[1] ~= nil or next(value) == nil)
end

local function __TS__ArrayConcat(self, ...)
    local items = {...}
    local result = {}
    local len = 0
    for i = 1, #self do
        len = len + 1
        result[len] = self[i]
    end
    for i = 1, #items do
        local item = items[i]
        if __TS__ArrayIsArray(item) then
            for j = 1, #item do
                len = len + 1
                result[len] = item[j]
            end
        else
            len = len + 1
            result[len] = item
        end
    end
    return result
end

local __TS__Symbol, Symbol
do
    local symbolMetatable = {__tostring = function(self)
        return ("Symbol(" .. (self.description or "")) .. ")"
    end}
    function __TS__Symbol(description)
        return setmetatable({description = description}, symbolMetatable)
    end
    Symbol = {
        asyncDispose = __TS__Symbol("Symbol.asyncDispose"),
        dispose = __TS__Symbol("Symbol.dispose"),
        iterator = __TS__Symbol("Symbol.iterator"),
        hasInstance = __TS__Symbol("Symbol.hasInstance"),
        species = __TS__Symbol("Symbol.species"),
        toStringTag = __TS__Symbol("Symbol.toStringTag")
    }
end

local function __TS__ArrayEntries(array)
    local key = 0
    return {
        [Symbol.iterator] = function(self)
            return self
        end,
        next = function(self)
            local result = {done = array[key + 1] == nil, value = {key, array[key + 1]}}
            key = key + 1
            return result
        end
    }
end

local function __TS__ArrayEvery(self, callbackfn, thisArg)
    for i = 1, #self do
        if not callbackfn(thisArg, self[i], i - 1, self) then
            return false
        end
    end
    return true
end

local function __TS__ArrayFill(self, value, start, ____end)
    local relativeStart = start or 0
    local relativeEnd = ____end or #self
    if relativeStart < 0 then
        relativeStart = relativeStart + #self
    end
    if relativeEnd < 0 then
        relativeEnd = relativeEnd + #self
    end
    do
        local i = relativeStart
        while i < relativeEnd do
            self[i + 1] = value
            i = i + 1
        end
    end
    return self
end

local function __TS__ArrayFilter(self, callbackfn, thisArg)
    local result = {}
    local len = 0
    for i = 1, #self do
        if callbackfn(thisArg, self[i], i - 1, self) then
            len = len + 1
            result[len] = self[i]
        end
    end
    return result
end

local function __TS__ArrayForEach(self, callbackFn, thisArg)
    for i = 1, #self do
        callbackFn(thisArg, self[i], i - 1, self)
    end
end

local function __TS__ArrayFind(self, predicate, thisArg)
    for i = 1, #self do
        local elem = self[i]
        if predicate(thisArg, elem, i - 1, self) then
            return elem
        end
    end
    return nil
end

local function __TS__ArrayFindIndex(self, callbackFn, thisArg)
    for i = 1, #self do
        if callbackFn(thisArg, self[i], i - 1, self) then
            return i - 1
        end
    end
    return -1
end

local __TS__Iterator
do
    local function iteratorGeneratorStep(self)
        local co = self.____coroutine
        local status, value = coroutine.resume(co)
        if not status then
            error(value, 0)
        end
        if coroutine.status(co) == "dead" then
            return
        end
        return true, value
    end
    local function iteratorIteratorStep(self)
        local result = self:next()
        if result.done then
            return
        end
        return true, result.value
    end
    local function iteratorStringStep(self, index)
        index = index + 1
        if index > #self then
            return
        end
        return index, string.sub(self, index, index)
    end
    function __TS__Iterator(iterable)
        if type(iterable) == "string" then
            return iteratorStringStep, iterable, 0
        elseif iterable.____coroutine ~= nil then
            return iteratorGeneratorStep, iterable
        elseif iterable[Symbol.iterator] then
            local iterator = iterable[Symbol.iterator](iterable)
            return iteratorIteratorStep, iterator
        else
            return ipairs(iterable)
        end
    end
end

local __TS__ArrayFrom
do
    local function arrayLikeStep(self, index)
        index = index + 1
        if index > self.length then
            return
        end
        return index, self[index]
    end
    local function arrayLikeIterator(arr)
        if type(arr.length) == "number" then
            return arrayLikeStep, arr, 0
        end
        return __TS__Iterator(arr)
    end
    function __TS__ArrayFrom(arrayLike, mapFn, thisArg)
        local result = {}
        if mapFn == nil then
            for ____, v in arrayLikeIterator(arrayLike) do
                result[#result + 1] = v
            end
        else
            for i, v in arrayLikeIterator(arrayLike) do
                result[#result + 1] = mapFn(thisArg, v, i - 1)
            end
        end
        return result
    end
end

local function __TS__ArrayIncludes(self, searchElement, fromIndex)
    if fromIndex == nil then
        fromIndex = 0
    end
    local len = #self
    local k = fromIndex
    if fromIndex < 0 then
        k = len + fromIndex
    end
    if k < 0 then
        k = 0
    end
    for i = k + 1, len do
        if self[i] == searchElement then
            return true
        end
    end
    return false
end

local function __TS__ArrayIndexOf(self, searchElement, fromIndex)
    if fromIndex == nil then
        fromIndex = 0
    end
    local len = #self
    if len == 0 then
        return -1
    end
    if fromIndex >= len then
        return -1
    end
    if fromIndex < 0 then
        fromIndex = len + fromIndex
        if fromIndex < 0 then
            fromIndex = 0
        end
    end
    for i = fromIndex + 1, len do
        if self[i] == searchElement then
            return i - 1
        end
    end
    return -1
end

local function __TS__ArrayJoin(self, separator)
    if separator == nil then
        separator = ","
    end
    local parts = {}
    for i = 1, #self do
        parts[i] = tostring(self[i])
    end
    return table.concat(parts, separator)
end

local function __TS__ArrayMap(self, callbackfn, thisArg)
    local result = {}
    for i = 1, #self do
        result[i] = callbackfn(thisArg, self[i], i - 1, self)
    end
    return result
end

local function __TS__ArrayPush(self, ...)
    local items = {...}
    local len = #self
    for i = 1, #items do
        len = len + 1
        self[len] = items[i]
    end
    return len
end

local function __TS__ArrayPushArray(self, items)
    local len = #self
    for i = 1, #items do
        len = len + 1
        self[len] = items[i]
    end
    return len
end

local function __TS__CountVarargs(...)
    return select("#", ...)
end

local function __TS__ArrayReduce(self, callbackFn, ...)
    local len = #self
    local k = 0
    local accumulator = nil
    if __TS__CountVarargs(...) ~= 0 then
        accumulator = ...
    elseif len > 0 then
        accumulator = self[1]
        k = 1
    else
        error("Reduce of empty array with no initial value", 0)
    end
    for i = k + 1, len do
        accumulator = callbackFn(
            nil,
            accumulator,
            self[i],
            i - 1,
            self
        )
    end
    return accumulator
end

local function __TS__ArrayReduceRight(self, callbackFn, ...)
    local len = #self
    local k = len - 1
    local accumulator = nil
    if __TS__CountVarargs(...) ~= 0 then
        accumulator = ...
    elseif len > 0 then
        accumulator = self[k + 1]
        k = k - 1
    else
        error("Reduce of empty array with no initial value", 0)
    end
    for i = k + 1, 1, -1 do
        accumulator = callbackFn(
            nil,
            accumulator,
            self[i],
            i - 1,
            self
        )
    end
    return accumulator
end

local function __TS__ArrayReverse(self)
    local i = 1
    local j = #self
    while i < j do
        local temp = self[j]
        self[j] = self[i]
        self[i] = temp
        i = i + 1
        j = j - 1
    end
    return self
end

local function __TS__ArrayUnshift(self, ...)
    local items = {...}
    local numItemsToInsert = #items
    if numItemsToInsert == 0 then
        return #self
    end
    for i = #self, 1, -1 do
        self[i + numItemsToInsert] = self[i]
    end
    for i = 1, numItemsToInsert do
        self[i] = items[i]
    end
    return #self
end

local function __TS__ArraySort(self, compareFn)
    if compareFn ~= nil then
        table.sort(
            self,
            function(a, b) return compareFn(nil, a, b) < 0 end
        )
    else
        table.sort(self)
    end
    return self
end

local function __TS__ArraySlice(self, first, last)
    local len = #self
    first = first or 0
    if first < 0 then
        first = len + first
        if first < 0 then
            first = 0
        end
    else
        if first > len then
            first = len
        end
    end
    last = last or len
    if last < 0 then
        last = len + last
        if last < 0 then
            last = 0
        end
    else
        if last > len then
            last = len
        end
    end
    local out = {}
    first = first + 1
    last = last + 1
    local n = 1
    while first < last do
        out[n] = self[first]
        first = first + 1
        n = n + 1
    end
    return out
end

local function __TS__ArraySome(self, callbackfn, thisArg)
    for i = 1, #self do
        if callbackfn(thisArg, self[i], i - 1, self) then
            return true
        end
    end
    return false
end

local function __TS__ArraySplice(self, ...)
    local args = {...}
    local len = #self
    local actualArgumentCount = __TS__CountVarargs(...)
    local start = args[1]
    local deleteCount = args[2]
    if start < 0 then
        start = len + start
        if start < 0 then
            start = 0
        end
    elseif start > len then
        start = len
    end
    local itemCount = actualArgumentCount - 2
    if itemCount < 0 then
        itemCount = 0
    end
    local actualDeleteCount
    if actualArgumentCount == 0 then
        actualDeleteCount = 0
    elseif actualArgumentCount == 1 then
        actualDeleteCount = len - start
    else
        actualDeleteCount = deleteCount or 0
        if actualDeleteCount < 0 then
            actualDeleteCount = 0
        end
        if actualDeleteCount > len - start then
            actualDeleteCount = len - start
        end
    end
    local out = {}
    for k = 1, actualDeleteCount do
        local from = start + k
        if self[from] ~= nil then
            out[k] = self[from]
        end
    end
    if itemCount < actualDeleteCount then
        for k = start + 1, len - actualDeleteCount do
            local from = k + actualDeleteCount
            local to = k + itemCount
            if self[from] then
                self[to] = self[from]
            else
                self[to] = nil
            end
        end
        for k = len - actualDeleteCount + itemCount + 1, len do
            self[k] = nil
        end
    elseif itemCount > actualDeleteCount then
        for k = len - actualDeleteCount, start + 1, -1 do
            local from = k + actualDeleteCount
            local to = k + itemCount
            if self[from] then
                self[to] = self[from]
            else
                self[to] = nil
            end
        end
    end
    local j = start + 1
    for i = 3, actualArgumentCount do
        self[j] = args[i]
        j = j + 1
    end
    for k = #self, len - actualDeleteCount + itemCount + 1, -1 do
        self[k] = nil
    end
    return out
end

local function __TS__ArrayToObject(self)
    local object = {}
    for i = 1, #self do
        object[i - 1] = self[i]
    end
    return object
end

local function __TS__ArrayFlat(self, depth)
    if depth == nil then
        depth = 1
    end
    local result = {}
    local len = 0
    for i = 1, #self do
        local value = self[i]
        if depth > 0 and __TS__ArrayIsArray(value) then
            local toAdd
            if depth == 1 then
                toAdd = value
            else
                toAdd = __TS__ArrayFlat(value, depth - 1)
            end
            for j = 1, #toAdd do
                local val = toAdd[j]
                len = len + 1
                result[len] = val
            end
        else
            len = len + 1
            result[len] = value
        end
    end
    return result
end

local function __TS__ArrayFlatMap(self, callback, thisArg)
    local result = {}
    local len = 0
    for i = 1, #self do
        local value = callback(thisArg, self[i], i - 1, self)
        if __TS__ArrayIsArray(value) then
            for j = 1, #value do
                len = len + 1
                result[len] = value[j]
            end
        else
            len = len + 1
            result[len] = value
        end
    end
    return result
end

local function __TS__ArraySetLength(self, length)
    if length < 0 or length ~= length or length == math.huge or math.floor(length) ~= length then
        error(
            "invalid array length: " .. tostring(length),
            0
        )
    end
    for i = length + 1, #self do
        self[i] = nil
    end
    return length
end

local __TS__Unpack = table.unpack or unpack

local function __TS__ArrayToReversed(self)
    local copy = {__TS__Unpack(self)}
    __TS__ArrayReverse(copy)
    return copy
end

local function __TS__ArrayToSorted(self, compareFn)
    local copy = {__TS__Unpack(self)}
    __TS__ArraySort(copy, compareFn)
    return copy
end

local function __TS__ArrayToSpliced(self, start, deleteCount, ...)
    local copy = {__TS__Unpack(self)}
    __TS__ArraySplice(copy, start, deleteCount, ...)
    return copy
end

local function __TS__ArrayWith(self, index, value)
    local copy = {__TS__Unpack(self)}
    copy[index + 1] = value
    return copy
end

local function __TS__New(target, ...)
    local instance = setmetatable({}, target.prototype)
    instance:____constructor(...)
    return instance
end

local function __TS__InstanceOf(obj, classTbl)
    if type(classTbl) ~= "table" then
        error("Right-hand side of 'instanceof' is not an object", 0)
    end
    if classTbl[Symbol.hasInstance] ~= nil then
        return not not classTbl[Symbol.hasInstance](classTbl, obj)
    end
    if type(obj) == "table" then
        local luaClass = obj.constructor
        while luaClass ~= nil do
            if luaClass == classTbl then
                return true
            end
            luaClass = luaClass.____super
        end
    end
    return false
end

local function __TS__Class(self)
    local c = {prototype = {}}
    c.prototype.__index = c.prototype
    c.prototype.constructor = c
    return c
end

local __TS__Promise
do
    local function makeDeferredPromiseFactory()
        local resolve
        local reject
        local function executor(____, res, rej)
            resolve = res
            reject = rej
        end
        return function()
            local promise = __TS__New(__TS__Promise, executor)
            return promise, resolve, reject
        end
    end
    local makeDeferredPromise = makeDeferredPromiseFactory()
    local function isPromiseLike(value)
        return __TS__InstanceOf(value, __TS__Promise)
    end
    local function doNothing(self)
    end
    local ____pcall = _G.pcall
    __TS__Promise = __TS__Class()
    __TS__Promise.name = "__TS__Promise"
    function __TS__Promise.prototype.____constructor(self, executor)
        self.state = 0
        self.fulfilledCallbacks = {}
        self.rejectedCallbacks = {}
        self.finallyCallbacks = {}
        local success, ____error = ____pcall(
            executor,
            nil,
            function(____, v) return self:resolve(v) end,
            function(____, err) return self:reject(err) end
        )
        if not success then
            self:reject(____error)
        end
    end
    function __TS__Promise.resolve(value)
        if __TS__InstanceOf(value, __TS__Promise) then
            return value
        end
        local promise = __TS__New(__TS__Promise, doNothing)
        promise.state = 1
        promise.value = value
        return promise
    end
    function __TS__Promise.reject(reason)
        local promise = __TS__New(__TS__Promise, doNothing)
        promise.state = 2
        promise.rejectionReason = reason
        return promise
    end
    __TS__Promise.prototype["then"] = function(self, onFulfilled, onRejected)
        local promise, resolve, reject = makeDeferredPromise()
        self:addCallbacks(
            onFulfilled and self:createPromiseResolvingCallback(onFulfilled, resolve, reject) or resolve,
            onRejected and self:createPromiseResolvingCallback(onRejected, resolve, reject) or reject
        )
        return promise
    end
    function __TS__Promise.prototype.addCallbacks(self, fulfilledCallback, rejectedCallback)
        if self.state == 1 then
            return fulfilledCallback(nil, self.value)
        end
        if self.state == 2 then
            return rejectedCallback(nil, self.rejectionReason)
        end
        local ____self_fulfilledCallbacks_0 = self.fulfilledCallbacks
        ____self_fulfilledCallbacks_0[#____self_fulfilledCallbacks_0 + 1] = fulfilledCallback
        local ____self_rejectedCallbacks_1 = self.rejectedCallbacks
        ____self_rejectedCallbacks_1[#____self_rejectedCallbacks_1 + 1] = rejectedCallback
    end
    function __TS__Promise.prototype.catch(self, onRejected)
        return self["then"](self, nil, onRejected)
    end
    function __TS__Promise.prototype.finally(self, onFinally)
        if onFinally then
            local ____self_finallyCallbacks_2 = self.finallyCallbacks
            ____self_finallyCallbacks_2[#____self_finallyCallbacks_2 + 1] = onFinally
            if self.state ~= 0 then
                onFinally(nil)
            end
        end
        return self
    end
    function __TS__Promise.prototype.resolve(self, value)
        if isPromiseLike(value) then
            return value:addCallbacks(
                function(____, v) return self:resolve(v) end,
                function(____, err) return self:reject(err) end
            )
        end
        if self.state == 0 then
            self.state = 1
            self.value = value
            return self:invokeCallbacks(self.fulfilledCallbacks, value)
        end
    end
    function __TS__Promise.prototype.reject(self, reason)
        if self.state == 0 then
            self.state = 2
            self.rejectionReason = reason
            return self:invokeCallbacks(self.rejectedCallbacks, reason)
        end
    end
    function __TS__Promise.prototype.invokeCallbacks(self, callbacks, value)
        local callbacksLength = #callbacks
        local finallyCallbacks = self.finallyCallbacks
        local finallyCallbacksLength = #finallyCallbacks
        if callbacksLength ~= 0 then
            for i = 1, callbacksLength - 1 do
                callbacks[i](callbacks, value)
            end
            if finallyCallbacksLength == 0 then
                return callbacks[callbacksLength](callbacks, value)
            end
            callbacks[callbacksLength](callbacks, value)
        end
        if finallyCallbacksLength ~= 0 then
            for i = 1, finallyCallbacksLength - 1 do
                finallyCallbacks[i](finallyCallbacks)
            end
            return finallyCallbacks[finallyCallbacksLength](finallyCallbacks)
        end
    end
    function __TS__Promise.prototype.createPromiseResolvingCallback(self, f, resolve, reject)
        return function(____, value)
            local success, resultOrError = ____pcall(f, nil, value)
            if not success then
                return reject(nil, resultOrError)
            end
            return self:handleCallbackValue(resultOrError, resolve, reject)
        end
    end
    function __TS__Promise.prototype.handleCallbackValue(self, value, resolve, reject)
        if isPromiseLike(value) then
            local nextpromise = value
            if nextpromise.state == 1 then
                return resolve(nil, nextpromise.value)
            elseif nextpromise.state == 2 then
                return reject(nil, nextpromise.rejectionReason)
            else
                return nextpromise:addCallbacks(resolve, reject)
            end
        else
            return resolve(nil, value)
        end
    end
end

local __TS__AsyncAwaiter, __TS__Await
do
    local ____coroutine = _G.coroutine or ({})
    local cocreate = ____coroutine.create
    local coresume = ____coroutine.resume
    local costatus = ____coroutine.status
    local coyield = ____coroutine.yield
    function __TS__AsyncAwaiter(generator)
        return __TS__New(
            __TS__Promise,
            function(____, resolve, reject)
                local fulfilled, step, resolved, asyncCoroutine
                function fulfilled(self, value)
                    local success, resultOrError = coresume(asyncCoroutine, value)
                    if success then
                        return step(resultOrError)
                    end
                    return reject(nil, resultOrError)
                end
                function step(result)
                    if resolved then
                        return
                    end
                    if costatus(asyncCoroutine) == "dead" then
                        return resolve(nil, result)
                    end
                    return __TS__Promise.resolve(result):addCallbacks(fulfilled, reject)
                end
                resolved = false
                asyncCoroutine = cocreate(generator)
                local success, resultOrError = coresume(
                    asyncCoroutine,
                    function(____, v)
                        resolved = true
                        return __TS__Promise.resolve(v):addCallbacks(resolve, reject)
                    end
                )
                if success then
                    return step(resultOrError)
                else
                    return reject(nil, resultOrError)
                end
            end
        )
    end
    function __TS__Await(thing)
        return coyield(thing)
    end
end

local function __TS__ClassExtends(target, base)
    target.____super = base
    local staticMetatable = setmetatable({__index = base}, base)
    setmetatable(target, staticMetatable)
    local baseMetatable = getmetatable(base)
    if baseMetatable then
        if type(baseMetatable.__index) == "function" then
            staticMetatable.__index = baseMetatable.__index
        end
        if type(baseMetatable.__newindex) == "function" then
            staticMetatable.__newindex = baseMetatable.__newindex
        end
    end
    setmetatable(target.prototype, base.prototype)
    if type(base.prototype.__index) == "function" then
        target.prototype.__index = base.prototype.__index
    end
    if type(base.prototype.__newindex) == "function" then
        target.prototype.__newindex = base.prototype.__newindex
    end
    if type(base.prototype.__tostring) == "function" then
        target.prototype.__tostring = base.prototype.__tostring
    end
end

local function __TS__CloneDescriptor(____bindingPattern0)
    local value
    local writable
    local set
    local get
    local configurable
    local enumerable
    enumerable = ____bindingPattern0.enumerable
    configurable = ____bindingPattern0.configurable
    get = ____bindingPattern0.get
    set = ____bindingPattern0.set
    writable = ____bindingPattern0.writable
    value = ____bindingPattern0.value
    local descriptor = {enumerable = enumerable == true, configurable = configurable == true}
    local hasGetterOrSetter = get ~= nil or set ~= nil
    local hasValueOrWritableAttribute = writable ~= nil or value ~= nil
    if hasGetterOrSetter and hasValueOrWritableAttribute then
        error("Invalid property descriptor. Cannot both specify accessors and a value or writable attribute.", 0)
    end
    if get or set then
        descriptor.get = get
        descriptor.set = set
    else
        descriptor.value = value
        descriptor.writable = writable == true
    end
    return descriptor
end

local function __TS__Decorate(self, originalValue, decorators, context)
    local result = originalValue
    do
        local i = #decorators
        while i >= 0 do
            local decorator = decorators[i + 1]
            if decorator ~= nil then
                local ____decorator_result_0 = decorator(self, result, context)
                if ____decorator_result_0 == nil then
                    ____decorator_result_0 = result
                end
                result = ____decorator_result_0
            end
            i = i - 1
        end
    end
    return result
end

local function __TS__ObjectAssign(target, ...)
    local sources = {...}
    for i = 1, #sources do
        local source = sources[i]
        for key in pairs(source) do
            target[key] = source[key]
        end
    end
    return target
end

local function __TS__ObjectGetOwnPropertyDescriptor(object, key)
    local metatable = getmetatable(object)
    if not metatable then
        return
    end
    if not rawget(metatable, "_descriptors") then
        return
    end
    return rawget(metatable, "_descriptors")[key]
end

local __TS__DescriptorGet
do
    local getmetatable = _G.getmetatable
    local ____rawget = _G.rawget
    function __TS__DescriptorGet(self, metatable, key)
        while metatable do
            local rawResult = ____rawget(metatable, key)
            if rawResult ~= nil then
                return rawResult
            end
            local descriptors = ____rawget(metatable, "_descriptors")
            if descriptors then
                local descriptor = descriptors[key]
                if descriptor ~= nil then
                    if descriptor.get then
                        return descriptor.get(self)
                    end
                    return descriptor.value
                end
            end
            metatable = getmetatable(metatable)
        end
    end
end

local __TS__DescriptorSet
do
    local getmetatable = _G.getmetatable
    local ____rawget = _G.rawget
    local rawset = _G.rawset
    function __TS__DescriptorSet(self, metatable, key, value)
        while metatable do
            local descriptors = ____rawget(metatable, "_descriptors")
            if descriptors then
                local descriptor = descriptors[key]
                if descriptor ~= nil then
                    if descriptor.set then
                        descriptor.set(self, value)
                    else
                        if descriptor.writable == false then
                            error(
                                ((("Cannot assign to read only property '" .. key) .. "' of object '") .. tostring(self)) .. "'",
                                0
                            )
                        end
                        descriptor.value = value
                    end
                    return
                end
            end
            metatable = getmetatable(metatable)
        end
        rawset(self, key, value)
    end
end

local __TS__SetDescriptor
do
    local getmetatable = _G.getmetatable
    local function descriptorIndex(self, key)
        return __TS__DescriptorGet(
            self,
            getmetatable(self),
            key
        )
    end
    local function descriptorNewIndex(self, key, value)
        return __TS__DescriptorSet(
            self,
            getmetatable(self),
            key,
            value
        )
    end
    function __TS__SetDescriptor(target, key, desc, isPrototype)
        if isPrototype == nil then
            isPrototype = false
        end
        local ____isPrototype_0
        if isPrototype then
            ____isPrototype_0 = target
        else
            ____isPrototype_0 = getmetatable(target)
        end
        local metatable = ____isPrototype_0
        if not metatable then
            metatable = {}
            setmetatable(target, metatable)
        end
        local value = rawget(target, key)
        if value ~= nil then
            rawset(target, key, nil)
        end
        if not rawget(metatable, "_descriptors") then
            metatable._descriptors = {}
        end
        metatable._descriptors[key] = __TS__CloneDescriptor(desc)
        metatable.__index = descriptorIndex
        metatable.__newindex = descriptorNewIndex
    end
end

local function __TS__DecorateLegacy(decorators, target, key, desc)
    local result = target
    do
        local i = #decorators
        while i >= 0 do
            local decorator = decorators[i + 1]
            if decorator ~= nil then
                local oldResult = result
                if key == nil then
                    result = decorator(nil, result)
                elseif desc == true then
                    local value = rawget(target, key)
                    local descriptor = __TS__ObjectGetOwnPropertyDescriptor(target, key) or ({configurable = true, writable = true, value = value})
                    local desc = decorator(nil, target, key, descriptor) or descriptor
                    local isSimpleValue = desc.configurable == true and desc.writable == true and not desc.get and not desc.set
                    if isSimpleValue then
                        rawset(target, key, desc.value)
                    else
                        __TS__SetDescriptor(
                            target,
                            key,
                            __TS__ObjectAssign({}, descriptor, desc)
                        )
                    end
                elseif desc == false then
                    result = decorator(nil, target, key, desc)
                else
                    result = decorator(nil, target, key)
                end
                result = result or oldResult
            end
            i = i - 1
        end
    end
    return result
end

local function __TS__DecorateParam(paramIndex, decorator)
    return function(____, target, key) return decorator(nil, target, key, paramIndex) end
end

local function __TS__StringIncludes(self, searchString, position)
    if not position then
        position = 1
    else
        position = position + 1
    end
    local index = string.find(self, searchString, position, true)
    return index ~= nil
end

local Error, RangeError, ReferenceError, SyntaxError, TypeError, URIError
do
    local function getErrorStack(self, constructor)
        if debug == nil then
            return nil
        end
        local level = 1
        while true do
            local info = debug.getinfo(level, "f")
            level = level + 1
            if not info then
                level = 1
                break
            elseif info.func == constructor then
                break
            end
        end
        if __TS__StringIncludes(_VERSION, "Lua 5.0") then
            return debug.traceback(("[Level " .. tostring(level)) .. "]")
        else
            return debug.traceback(nil, level)
        end
    end
    local function wrapErrorToString(self, getDescription)
        return function(self)
            local description = getDescription(self)
            local caller = debug.getinfo(3, "f")
            local isClassicLua = __TS__StringIncludes(_VERSION, "Lua 5.0") or _VERSION == "Lua 5.1"
            if isClassicLua or caller and caller.func ~= error then
                return description
            else
                return (description .. "\n") .. tostring(self.stack)
            end
        end
    end
    local function initErrorClass(self, Type, name)
        Type.name = name
        return setmetatable(
            Type,
            {__call = function(____, _self, message) return __TS__New(Type, message) end}
        )
    end
    local ____initErrorClass_1 = initErrorClass
    local ____class_0 = __TS__Class()
    ____class_0.name = ""
    function ____class_0.prototype.____constructor(self, message)
        if message == nil then
            message = ""
        end
        self.message = message
        self.name = "Error"
        self.stack = getErrorStack(nil, self.constructor.new)
        local metatable = getmetatable(self)
        if metatable and not metatable.__errorToStringPatched then
            metatable.__errorToStringPatched = true
            metatable.__tostring = wrapErrorToString(nil, metatable.__tostring)
        end
    end
    function ____class_0.prototype.__tostring(self)
        return self.message ~= "" and (self.name .. ": ") .. self.message or self.name
    end
    Error = ____initErrorClass_1(nil, ____class_0, "Error")
    local function createErrorClass(self, name)
        local ____initErrorClass_3 = initErrorClass
        local ____class_2 = __TS__Class()
        ____class_2.name = ____class_2.name
        __TS__ClassExtends(____class_2, Error)
        function ____class_2.prototype.____constructor(self, ...)
            ____class_2.____super.prototype.____constructor(self, ...)
            self.name = name
        end
        return ____initErrorClass_3(nil, ____class_2, name)
    end
    RangeError = createErrorClass(nil, "RangeError")
    ReferenceError = createErrorClass(nil, "ReferenceError")
    SyntaxError = createErrorClass(nil, "SyntaxError")
    TypeError = createErrorClass(nil, "TypeError")
    URIError = createErrorClass(nil, "URIError")
end

local function __TS__ObjectGetOwnPropertyDescriptors(object)
    local metatable = getmetatable(object)
    if not metatable then
        return {}
    end
    return rawget(metatable, "_descriptors") or ({})
end

local function __TS__Delete(target, key)
    local descriptors = __TS__ObjectGetOwnPropertyDescriptors(target)
    local descriptor = descriptors[key]
    if descriptor then
        if not descriptor.configurable then
            error(
                __TS__New(
                    TypeError,
                    ((("Cannot delete property " .. tostring(key)) .. " of ") .. tostring(target)) .. "."
                ),
                0
            )
        end
        descriptors[key] = nil
        return true
    end
    target[key] = nil
    return true
end

local function __TS__StringAccess(self, index)
    if index >= 0 and index < #self then
        return string.sub(self, index + 1, index + 1)
    end
end

local function __TS__DelegatedYield(iterable)
    if type(iterable) == "string" then
        for index = 0, #iterable - 1 do
            coroutine.yield(__TS__StringAccess(iterable, index))
        end
    elseif iterable.____coroutine ~= nil then
        local co = iterable.____coroutine
        while true do
            local status, value = coroutine.resume(co)
            if not status then
                error(value, 0)
            end
            if coroutine.status(co) == "dead" then
                return value
            else
                coroutine.yield(value)
            end
        end
    elseif iterable[Symbol.iterator] then
        local iterator = iterable[Symbol.iterator](iterable)
        while true do
            local result = iterator:next()
            if result.done then
                return result.value
            else
                coroutine.yield(result.value)
            end
        end
    else
        for ____, value in ipairs(iterable) do
            coroutine.yield(value)
        end
    end
end

local function __TS__FunctionBind(fn, ...)
    local boundArgs = {...}
    return function(____, ...)
        local args = {...}
        __TS__ArrayUnshift(
            args,
            __TS__Unpack(boundArgs)
        )
        return fn(__TS__Unpack(args))
    end
end

local __TS__Generator
do
    local function generatorIterator(self)
        return self
    end
    local function generatorNext(self, ...)
        local co = self.____coroutine
        if coroutine.status(co) == "dead" then
            return {done = true}
        end
        local status, value = coroutine.resume(co, ...)
        if not status then
            error(value, 0)
        end
        return {
            value = value,
            done = coroutine.status(co) == "dead"
        }
    end
    function __TS__Generator(fn)
        return function(...)
            local args = {...}
            local argsLength = __TS__CountVarargs(...)
            return {
                ____coroutine = coroutine.create(function() return fn(__TS__Unpack(args, 1, argsLength)) end),
                [Symbol.iterator] = generatorIterator,
                next = generatorNext
            }
        end
    end
end

local function __TS__InstanceOfObject(value)
    local valueType = type(value)
    return valueType == "table" or valueType == "function"
end

local function __TS__LuaIteratorSpread(self, state, firstKey)
    local results = {}
    local key, value = self(state, firstKey)
    while key do
        results[#results + 1] = {key, value}
        key, value = self(state, key)
    end
    return __TS__Unpack(results)
end

local Map
do
    Map = __TS__Class()
    Map.name = "Map"
    function Map.prototype.____constructor(self, entries)
        self[Symbol.toStringTag] = "Map"
        self.items = {}
        self.size = 0
        self.nextKey = {}
        self.previousKey = {}
        if entries == nil then
            return
        end
        local iterable = entries
        if iterable[Symbol.iterator] then
            local iterator = iterable[Symbol.iterator](iterable)
            while true do
                local result = iterator:next()
                if result.done then
                    break
                end
                local value = result.value
                self:set(value[1], value[2])
            end
        else
            local array = entries
            for ____, kvp in ipairs(array) do
                self:set(kvp[1], kvp[2])
            end
        end
    end
    function Map.prototype.clear(self)
        self.items = {}
        self.nextKey = {}
        self.previousKey = {}
        self.firstKey = nil
        self.lastKey = nil
        self.size = 0
    end
    function Map.prototype.delete(self, key)
        local contains = self:has(key)
        if contains then
            self.size = self.size - 1
            local next = self.nextKey[key]
            local previous = self.previousKey[key]
            if next ~= nil and previous ~= nil then
                self.nextKey[previous] = next
                self.previousKey[next] = previous
            elseif next ~= nil then
                self.firstKey = next
                self.previousKey[next] = nil
            elseif previous ~= nil then
                self.lastKey = previous
                self.nextKey[previous] = nil
            else
                self.firstKey = nil
                self.lastKey = nil
            end
            self.nextKey[key] = nil
            self.previousKey[key] = nil
        end
        self.items[key] = nil
        return contains
    end
    function Map.prototype.forEach(self, callback)
        for ____, key in __TS__Iterator(self:keys()) do
            callback(nil, self.items[key], key, self)
        end
    end
    function Map.prototype.get(self, key)
        return self.items[key]
    end
    function Map.prototype.has(self, key)
        return self.nextKey[key] ~= nil or self.lastKey == key
    end
    function Map.prototype.set(self, key, value)
        local isNewValue = not self:has(key)
        if isNewValue then
            self.size = self.size + 1
        end
        self.items[key] = value
        if self.firstKey == nil then
            self.firstKey = key
            self.lastKey = key
        elseif isNewValue then
            self.nextKey[self.lastKey] = key
            self.previousKey[key] = self.lastKey
            self.lastKey = key
        end
        return self
    end
    Map.prototype[Symbol.iterator] = function(self)
        return self:entries()
    end
    function Map.prototype.entries(self)
        local items = self.items
        local nextKey = self.nextKey
        local key = self.firstKey
        return {
            [Symbol.iterator] = function(self)
                return self
            end,
            next = function(self)
                local result = {done = not key, value = {key, items[key]}}
                key = nextKey[key]
                return result
            end
        }
    end
    function Map.prototype.keys(self)
        local nextKey = self.nextKey
        local key = self.firstKey
        return {
            [Symbol.iterator] = function(self)
                return self
            end,
            next = function(self)
                local result = {done = not key, value = key}
                key = nextKey[key]
                return result
            end
        }
    end
    function Map.prototype.values(self)
        local items = self.items
        local nextKey = self.nextKey
        local key = self.firstKey
        return {
            [Symbol.iterator] = function(self)
                return self
            end,
            next = function(self)
                local result = {done = not key, value = items[key]}
                key = nextKey[key]
                return result
            end
        }
    end
    Map[Symbol.species] = Map
end

local function __TS__MapGroupBy(items, keySelector)
    local result = __TS__New(Map)
    local i = 0
    for ____, item in __TS__Iterator(items) do
        local key = keySelector(nil, item, i)
        if result:has(key) then
            local ____temp_0 = result:get(key)
            ____temp_0[#____temp_0 + 1] = item
        else
            result:set(key, {item})
        end
        i = i + 1
    end
    return result
end

local __TS__Match = string.match

local __TS__MathAtan2 = math.atan2 or math.atan

local __TS__MathModf = math.modf

local function __TS__NumberIsNaN(value)
    return value ~= value
end

local function __TS__MathSign(val)
    if __TS__NumberIsNaN(val) or val == 0 then
        return val
    end
    if val < 0 then
        return -1
    end
    return 1
end

local function __TS__NumberIsFinite(value)
    return type(value) == "number" and value == value and value ~= math.huge and value ~= -math.huge
end

local function __TS__MathTrunc(val)
    if not __TS__NumberIsFinite(val) or val == 0 then
        return val
    end
    return val > 0 and math.floor(val) or math.ceil(val)
end

local function __TS__Number(value)
    local valueType = type(value)
    if valueType == "number" then
        return value
    elseif valueType == "string" then
        local numberValue = tonumber(value)
        if numberValue then
            return numberValue
        end
        if value == "Infinity" then
            return math.huge
        end
        if value == "-Infinity" then
            return -math.huge
        end
        local stringWithoutSpaces = string.gsub(value, "%s", "")
        if stringWithoutSpaces == "" then
            return 0
        end
        return 0 / 0
    elseif valueType == "boolean" then
        return value and 1 or 0
    else
        return 0 / 0
    end
end

local function __TS__NumberIsInteger(value)
    return __TS__NumberIsFinite(value) and math.floor(value) == value
end

local function __TS__StringSubstring(self, start, ____end)
    if ____end ~= ____end then
        ____end = 0
    end
    if ____end ~= nil and start > ____end then
        start, ____end = ____end, start
    end
    if start >= 0 then
        start = start + 1
    else
        start = 1
    end
    if ____end ~= nil and ____end < 0 then
        ____end = 0
    end
    return string.sub(self, start, ____end)
end

local __TS__ParseInt
do
    local parseIntBasePattern = "0123456789aAbBcCdDeEfFgGhHiIjJkKlLmMnNoOpPqQrRsStTvVwWxXyYzZ"
    function __TS__ParseInt(numberString, base)
        if base == nil then
            base = 10
            local hexMatch = __TS__Match(numberString, "^%s*-?0[xX]")
            if hexMatch ~= nil then
                base = 16
                numberString = (__TS__Match(hexMatch, "-")) and "-" .. __TS__StringSubstring(numberString, #hexMatch) or __TS__StringSubstring(numberString, #hexMatch)
            end
        end
        if base < 2 or base > 36 then
            return 0 / 0
        end
        local allowedDigits = base <= 10 and __TS__StringSubstring(parseIntBasePattern, 0, base) or __TS__StringSubstring(parseIntBasePattern, 0, 10 + 2 * (base - 10))
        local pattern = ("^%s*(-?[" .. allowedDigits) .. "]*)"
        local number = tonumber((__TS__Match(numberString, pattern)), base)
        if number == nil then
            return 0 / 0
        end
        if number >= 0 then
            return math.floor(number)
        else
            return math.ceil(number)
        end
    end
end

local function __TS__ParseFloat(numberString)
    local infinityMatch = __TS__Match(numberString, "^%s*(-?Infinity)")
    if infinityMatch ~= nil then
        return __TS__StringAccess(infinityMatch, 0) == "-" and -math.huge or math.huge
    end
    local number = tonumber((__TS__Match(numberString, "^%s*(-?%d+%.?%d*)")))
    return number or 0 / 0
end

local __TS__NumberToString
do
    local radixChars = "0123456789abcdefghijklmnopqrstuvwxyz"
    function __TS__NumberToString(self, radix)
        if radix == nil or radix == 10 or self == math.huge or self == -math.huge or self ~= self then
            return tostring(self)
        end
        radix = math.floor(radix)
        if radix < 2 or radix > 36 then
            error("toString() radix argument must be between 2 and 36", 0)
        end
        local integer, fraction = __TS__MathModf(math.abs(self))
        local result = ""
        if radix == 8 then
            result = string.format("%o", integer)
        elseif radix == 16 then
            result = string.format("%x", integer)
        else
            repeat
                do
                    result = __TS__StringAccess(radixChars, integer % radix) .. result
                    integer = math.floor(integer / radix)
                end
            until not (integer ~= 0)
        end
        if fraction ~= 0 then
            result = result .. "."
            local delta = 1e-16
            repeat
                do
                    fraction = fraction * radix
                    delta = delta * radix
                    local digit = math.floor(fraction)
                    result = result .. __TS__StringAccess(radixChars, digit)
                    fraction = fraction - digit
                end
            until not (fraction >= delta)
        end
        if self < 0 then
            result = "-" .. result
        end
        return result
    end
end

local function __TS__NumberToFixed(self, fractionDigits)
    if math.abs(self) >= 1e+21 or self ~= self then
        return tostring(self)
    end
    local f = math.floor(fractionDigits or 0)
    if f < 0 or f > 99 then
        error("toFixed() digits argument must be between 0 and 99", 0)
    end
    return string.format(
        ("%." .. tostring(f)) .. "f",
        self
    )
end

local function __TS__ObjectDefineProperty(target, key, desc)
    local luaKey = type(key) == "number" and key + 1 or key
    local value = rawget(target, luaKey)
    local hasGetterOrSetter = desc.get ~= nil or desc.set ~= nil
    local descriptor
    if hasGetterOrSetter then
        if value ~= nil then
            error(
                "Cannot redefine property: " .. tostring(key),
                0
            )
        end
        descriptor = desc
    else
        local valueExists = value ~= nil
        local ____desc_set_4 = desc.set
        local ____desc_get_5 = desc.get
        local ____temp_0
        if desc.configurable ~= nil then
            ____temp_0 = desc.configurable
        else
            ____temp_0 = valueExists
        end
        local ____temp_1
        if desc.enumerable ~= nil then
            ____temp_1 = desc.enumerable
        else
            ____temp_1 = valueExists
        end
        local ____temp_2
        if desc.writable ~= nil then
            ____temp_2 = desc.writable
        else
            ____temp_2 = valueExists
        end
        local ____temp_3
        if desc.value ~= nil then
            ____temp_3 = desc.value
        else
            ____temp_3 = value
        end
        descriptor = {
            set = ____desc_set_4,
            get = ____desc_get_5,
            configurable = ____temp_0,
            enumerable = ____temp_1,
            writable = ____temp_2,
            value = ____temp_3
        }
    end
    __TS__SetDescriptor(target, luaKey, descriptor)
    return target
end

local function __TS__ObjectEntries(obj)
    local result = {}
    local len = 0
    for key in pairs(obj) do
        len = len + 1
        result[len] = {key, obj[key]}
    end
    return result
end

local function __TS__ObjectFromEntries(entries)
    local obj = {}
    local iterable = entries
    if iterable[Symbol.iterator] then
        local iterator = iterable[Symbol.iterator](iterable)
        while true do
            local result = iterator:next()
            if result.done then
                break
            end
            local value = result.value
            obj[value[1]] = value[2]
        end
    else
        for ____, entry in ipairs(entries) do
            obj[entry[1]] = entry[2]
        end
    end
    return obj
end

local function __TS__ObjectGroupBy(items, keySelector)
    local result = {}
    local i = 0
    for ____, item in __TS__Iterator(items) do
        local key = keySelector(nil, item, i)
        if result[key] ~= nil then
            local ____result_key_0 = result[key]
            ____result_key_0[#____result_key_0 + 1] = item
        else
            result[key] = {item}
        end
        i = i + 1
    end
    return result
end

local function __TS__ObjectKeys(obj)
    local result = {}
    local len = 0
    for key in pairs(obj) do
        len = len + 1
        result[len] = key
    end
    return result
end

local function __TS__ObjectRest(target, usedProperties)
    local result = {}
    for property in pairs(target) do
        if not usedProperties[property] then
            result[property] = target[property]
        end
    end
    return result
end

local function __TS__ObjectValues(obj)
    local result = {}
    local len = 0
    for key in pairs(obj) do
        len = len + 1
        result[len] = obj[key]
    end
    return result
end

local function __TS__PromiseAll(iterable)
    local results = {}
    local toResolve = {}
    local numToResolve = 0
    local i = 0
    for ____, item in __TS__Iterator(iterable) do
        if __TS__InstanceOf(item, __TS__Promise) then
            if item.state == 1 then
                results[i + 1] = item.value
            elseif item.state == 2 then
                return __TS__Promise.reject(item.rejectionReason)
            else
                numToResolve = numToResolve + 1
                toResolve[i] = item
            end
        else
            results[i + 1] = item
        end
        i = i + 1
    end
    if numToResolve == 0 then
        return __TS__Promise.resolve(results)
    end
    return __TS__New(
        __TS__Promise,
        function(____, resolve, reject)
            for index, promise in pairs(toResolve) do
                promise["then"](
                    promise,
                    function(____, data)
                        results[index + 1] = data
                        numToResolve = numToResolve - 1
                        if numToResolve == 0 then
                            resolve(nil, results)
                        end
                    end,
                    function(____, reason)
                        reject(nil, reason)
                    end
                )
            end
        end
    )
end

local function __TS__PromiseAllSettled(iterable)
    local results = {}
    local toResolve = {}
    local numToResolve = 0
    local i = 0
    for ____, item in __TS__Iterator(iterable) do
        if __TS__InstanceOf(item, __TS__Promise) then
            if item.state == 1 then
                results[i + 1] = {status = "fulfilled", value = item.value}
            elseif item.state == 2 then
                results[i + 1] = {status = "rejected", reason = item.rejectionReason}
            else
                numToResolve = numToResolve + 1
                toResolve[i] = item
            end
        else
            results[i + 1] = {status = "fulfilled", value = item}
        end
        i = i + 1
    end
    if numToResolve == 0 then
        return __TS__Promise.resolve(results)
    end
    return __TS__New(
        __TS__Promise,
        function(____, resolve)
            for index, promise in pairs(toResolve) do
                promise["then"](
                    promise,
                    function(____, data)
                        results[index + 1] = {status = "fulfilled", value = data}
                        numToResolve = numToResolve - 1
                        if numToResolve == 0 then
                            resolve(nil, results)
                        end
                    end,
                    function(____, reason)
                        results[index + 1] = {status = "rejected", reason = reason}
                        numToResolve = numToResolve - 1
                        if numToResolve == 0 then
                            resolve(nil, results)
                        end
                    end
                )
            end
        end
    )
end

local function __TS__PromiseAny(iterable)
    local rejections = {}
    local pending = {}
    for ____, item in __TS__Iterator(iterable) do
        if __TS__InstanceOf(item, __TS__Promise) then
            if item.state == 1 then
                return __TS__Promise.resolve(item.value)
            elseif item.state == 2 then
                rejections[#rejections + 1] = item.rejectionReason
            else
                pending[#pending + 1] = item
            end
        else
            return __TS__Promise.resolve(item)
        end
    end
    if #pending == 0 then
        return __TS__Promise.reject("No promises to resolve with .any()")
    end
    local numResolved = 0
    return __TS__New(
        __TS__Promise,
        function(____, resolve, reject)
            for ____, promise in ipairs(pending) do
                promise["then"](
                    promise,
                    function(____, data)
                        resolve(nil, data)
                    end,
                    function(____, reason)
                        rejections[#rejections + 1] = reason
                        numResolved = numResolved + 1
                        if numResolved == #pending then
                            reject(nil, {name = "AggregateError", message = "All Promises rejected", errors = rejections})
                        end
                    end
                )
            end
        end
    )
end

local function __TS__PromiseRace(iterable)
    local pending = {}
    for ____, item in __TS__Iterator(iterable) do
        if __TS__InstanceOf(item, __TS__Promise) then
            if item.state == 1 then
                return __TS__Promise.resolve(item.value)
            elseif item.state == 2 then
                return __TS__Promise.reject(item.rejectionReason)
            else
                pending[#pending + 1] = item
            end
        else
            return __TS__Promise.resolve(item)
        end
    end
    return __TS__New(
        __TS__Promise,
        function(____, resolve, reject)
            for ____, promise in ipairs(pending) do
                promise["then"](
                    promise,
                    function(____, value) return resolve(nil, value) end,
                    function(____, reason) return reject(nil, reason) end
                )
            end
        end
    )
end

local Set
do
    Set = __TS__Class()
    Set.name = "Set"
    function Set.prototype.____constructor(self, values)
        self[Symbol.toStringTag] = "Set"
        self.size = 0
        self.nextKey = {}
        self.previousKey = {}
        if values == nil then
            return
        end
        local iterable = values
        if iterable[Symbol.iterator] then
            local iterator = iterable[Symbol.iterator](iterable)
            while true do
                local result = iterator:next()
                if result.done then
                    break
                end
                self:add(result.value)
            end
        else
            local array = values
            for ____, value in ipairs(array) do
                self:add(value)
            end
        end
    end
    function Set.prototype.add(self, value)
        local isNewValue = not self:has(value)
        if isNewValue then
            self.size = self.size + 1
        end
        if self.firstKey == nil then
            self.firstKey = value
            self.lastKey = value
        elseif isNewValue then
            self.nextKey[self.lastKey] = value
            self.previousKey[value] = self.lastKey
            self.lastKey = value
        end
        return self
    end
    function Set.prototype.clear(self)
        self.nextKey = {}
        self.previousKey = {}
        self.firstKey = nil
        self.lastKey = nil
        self.size = 0
    end
    function Set.prototype.delete(self, value)
        local contains = self:has(value)
        if contains then
            self.size = self.size - 1
            local next = self.nextKey[value]
            local previous = self.previousKey[value]
            if next ~= nil and previous ~= nil then
                self.nextKey[previous] = next
                self.previousKey[next] = previous
            elseif next ~= nil then
                self.firstKey = next
                self.previousKey[next] = nil
            elseif previous ~= nil then
                self.lastKey = previous
                self.nextKey[previous] = nil
            else
                self.firstKey = nil
                self.lastKey = nil
            end
            self.nextKey[value] = nil
            self.previousKey[value] = nil
        end
        return contains
    end
    function Set.prototype.forEach(self, callback)
        for ____, key in __TS__Iterator(self:keys()) do
            callback(nil, key, key, self)
        end
    end
    function Set.prototype.has(self, value)
        return self.nextKey[value] ~= nil or self.lastKey == value
    end
    Set.prototype[Symbol.iterator] = function(self)
        return self:values()
    end
    function Set.prototype.entries(self)
        local nextKey = self.nextKey
        local key = self.firstKey
        return {
            [Symbol.iterator] = function(self)
                return self
            end,
            next = function(self)
                local result = {done = not key, value = {key, key}}
                key = nextKey[key]
                return result
            end
        }
    end
    function Set.prototype.keys(self)
        local nextKey = self.nextKey
        local key = self.firstKey
        return {
            [Symbol.iterator] = function(self)
                return self
            end,
            next = function(self)
                local result = {done = not key, value = key}
                key = nextKey[key]
                return result
            end
        }
    end
    function Set.prototype.values(self)
        local nextKey = self.nextKey
        local key = self.firstKey
        return {
            [Symbol.iterator] = function(self)
                return self
            end,
            next = function(self)
                local result = {done = not key, value = key}
                key = nextKey[key]
                return result
            end
        }
    end
    function Set.prototype.union(self, other)
        local result = __TS__New(Set, self)
        for ____, item in __TS__Iterator(other) do
            result:add(item)
        end
        return result
    end
    function Set.prototype.intersection(self, other)
        local result = __TS__New(Set)
        for ____, item in __TS__Iterator(self) do
            if other:has(item) then
                result:add(item)
            end
        end
        return result
    end
    function Set.prototype.difference(self, other)
        local result = __TS__New(Set, self)
        for ____, item in __TS__Iterator(other) do
            result:delete(item)
        end
        return result
    end
    function Set.prototype.symmetricDifference(self, other)
        local result = __TS__New(Set, self)
        for ____, item in __TS__Iterator(other) do
            if self:has(item) then
                result:delete(item)
            else
                result:add(item)
            end
        end
        return result
    end
    function Set.prototype.isSubsetOf(self, other)
        for ____, item in __TS__Iterator(self) do
            if not other:has(item) then
                return false
            end
        end
        return true
    end
    function Set.prototype.isSupersetOf(self, other)
        for ____, item in __TS__Iterator(other) do
            if not self:has(item) then
                return false
            end
        end
        return true
    end
    function Set.prototype.isDisjointFrom(self, other)
        for ____, item in __TS__Iterator(self) do
            if other:has(item) then
                return false
            end
        end
        return true
    end
    Set[Symbol.species] = Set
end

local function __TS__SparseArrayNew(...)
    local sparseArray = {...}
    sparseArray.sparseLength = __TS__CountVarargs(...)
    return sparseArray
end

local function __TS__SparseArrayPush(sparseArray, ...)
    local args = {...}
    local argsLen = __TS__CountVarargs(...)
    local listLen = sparseArray.sparseLength
    for i = 1, argsLen do
        sparseArray[listLen + i] = args[i]
    end
    sparseArray.sparseLength = listLen + argsLen
end

local function __TS__SparseArraySpread(sparseArray)
    local _unpack = unpack or table.unpack
    return _unpack(sparseArray, 1, sparseArray.sparseLength)
end

local WeakMap
do
    WeakMap = __TS__Class()
    WeakMap.name = "WeakMap"
    function WeakMap.prototype.____constructor(self, entries)
        self[Symbol.toStringTag] = "WeakMap"
        self.items = {}
        setmetatable(self.items, {__mode = "k"})
        if entries == nil then
            return
        end
        local iterable = entries
        if iterable[Symbol.iterator] then
            local iterator = iterable[Symbol.iterator](iterable)
            while true do
                local result = iterator:next()
                if result.done then
                    break
                end
                local value = result.value
                self.items[value[1]] = value[2]
            end
        else
            for ____, kvp in ipairs(entries) do
                self.items[kvp[1]] = kvp[2]
            end
        end
    end
    function WeakMap.prototype.delete(self, key)
        local contains = self:has(key)
        self.items[key] = nil
        return contains
    end
    function WeakMap.prototype.get(self, key)
        return self.items[key]
    end
    function WeakMap.prototype.has(self, key)
        return self.items[key] ~= nil
    end
    function WeakMap.prototype.set(self, key, value)
        self.items[key] = value
        return self
    end
    WeakMap[Symbol.species] = WeakMap
end

local WeakSet
do
    WeakSet = __TS__Class()
    WeakSet.name = "WeakSet"
    function WeakSet.prototype.____constructor(self, values)
        self[Symbol.toStringTag] = "WeakSet"
        self.items = {}
        setmetatable(self.items, {__mode = "k"})
        if values == nil then
            return
        end
        local iterable = values
        if iterable[Symbol.iterator] then
            local iterator = iterable[Symbol.iterator](iterable)
            while true do
                local result = iterator:next()
                if result.done then
                    break
                end
                self.items[result.value] = true
            end
        else
            for ____, value in ipairs(values) do
                self.items[value] = true
            end
        end
    end
    function WeakSet.prototype.add(self, value)
        self.items[value] = true
        return self
    end
    function WeakSet.prototype.delete(self, value)
        local contains = self:has(value)
        self.items[value] = nil
        return contains
    end
    function WeakSet.prototype.has(self, value)
        return self.items[value] == true
    end
    WeakSet[Symbol.species] = WeakSet
end

local function __TS__SourceMapTraceBack(fileName, sourceMap)
    _G.__TS__sourcemap = _G.__TS__sourcemap or ({})
    _G.__TS__sourcemap[fileName] = sourceMap
    if _G.__TS__originalTraceback == nil then
        local originalTraceback = debug.traceback
        _G.__TS__originalTraceback = originalTraceback
        debug.traceback = function(thread, message, level)
            local trace
            if thread == nil and message == nil and level == nil then
                trace = originalTraceback()
            elseif __TS__StringIncludes(_VERSION, "Lua 5.0") then
                trace = originalTraceback((("[Level " .. tostring(level)) .. "] ") .. tostring(message))
            else
                trace = originalTraceback(thread, message, level)
            end
            if type(trace) ~= "string" then
                return trace
            end
            local function replacer(____, file, srcFile, line)
                local fileSourceMap = _G.__TS__sourcemap[file]
                if fileSourceMap ~= nil and fileSourceMap[line] ~= nil then
                    local data = fileSourceMap[line]
                    if type(data) == "number" then
                        return (srcFile .. ":") .. tostring(data)
                    end
                    return (data.file .. ":") .. tostring(data.line)
                end
                return (file .. ":") .. line
            end
            local result = string.gsub(
                trace,
                "(%S+)%.lua:(%d+)",
                function(file, line) return replacer(nil, file .. ".lua", file .. ".ts", line) end
            )
            local function stringReplacer(____, file, line)
                local fileSourceMap = _G.__TS__sourcemap[file]
                if fileSourceMap ~= nil and fileSourceMap[line] ~= nil then
                    local chunkName = (__TS__Match(file, "%[string \"([^\"]+)\"%]"))
                    local sourceName = string.gsub(chunkName, ".lua$", ".ts")
                    local data = fileSourceMap[line]
                    if type(data) == "number" then
                        return (sourceName .. ":") .. tostring(data)
                    end
                    return (data.file .. ":") .. tostring(data.line)
                end
                return (file .. ":") .. line
            end
            result = string.gsub(
                result,
                "(%[string \"[^\"]+\"%]):(%d+)",
                function(file, line) return stringReplacer(nil, file, line) end
            )
            return result
        end
    end
end

local function __TS__Spread(iterable)
    local arr = {}
    if type(iterable) == "string" then
        for i = 0, #iterable - 1 do
            arr[i + 1] = __TS__StringAccess(iterable, i)
        end
    else
        local len = 0
        for ____, item in __TS__Iterator(iterable) do
            len = len + 1
            arr[len] = item
        end
    end
    return __TS__Unpack(arr)
end

local function __TS__StringCharAt(self, pos)
    if pos ~= pos then
        pos = 0
    end
    if pos < 0 then
        return ""
    end
    return string.sub(self, pos + 1, pos + 1)
end

local function __TS__StringCharCodeAt(self, index)
    if index ~= index then
        index = 0
    end
    if index < 0 then
        return 0 / 0
    end
    return string.byte(self, index + 1) or 0 / 0
end

local function __TS__StringEndsWith(self, searchString, endPosition)
    if endPosition == nil or endPosition > #self then
        endPosition = #self
    end
    return string.sub(self, endPosition - #searchString + 1, endPosition) == searchString
end

local function __TS__StringPadEnd(self, maxLength, fillString)
    if fillString == nil then
        fillString = " "
    end
    if maxLength ~= maxLength then
        maxLength = 0
    end
    if maxLength == -math.huge or maxLength == math.huge then
        error("Invalid string length", 0)
    end
    if #self >= maxLength or #fillString == 0 then
        return self
    end
    maxLength = maxLength - #self
    if maxLength > #fillString then
        fillString = fillString .. string.rep(
            fillString,
            math.floor(maxLength / #fillString)
        )
    end
    return self .. string.sub(
        fillString,
        1,
        math.floor(maxLength)
    )
end

local function __TS__StringPadStart(self, maxLength, fillString)
    if fillString == nil then
        fillString = " "
    end
    if maxLength ~= maxLength then
        maxLength = 0
    end
    if maxLength == -math.huge or maxLength == math.huge then
        error("Invalid string length", 0)
    end
    if #self >= maxLength or #fillString == 0 then
        return self
    end
    maxLength = maxLength - #self
    if maxLength > #fillString then
        fillString = fillString .. string.rep(
            fillString,
            math.floor(maxLength / #fillString)
        )
    end
    return string.sub(
        fillString,
        1,
        math.floor(maxLength)
    ) .. self
end

local __TS__StringReplace
do
    local sub = string.sub
    function __TS__StringReplace(source, searchValue, replaceValue)
        local startPos, endPos = string.find(source, searchValue, nil, true)
        if not startPos then
            return source
        end
        local before = sub(source, 1, startPos - 1)
        local replacement = type(replaceValue) == "string" and replaceValue or replaceValue(nil, searchValue, startPos - 1, source)
        local after = sub(source, endPos + 1)
        return (before .. replacement) .. after
    end
end

local __TS__StringSplit
do
    local sub = string.sub
    local find = string.find
    function __TS__StringSplit(source, separator, limit)
        if limit == nil then
            limit = 4294967295
        end
        if limit == 0 then
            return {}
        end
        local result = {}
        local resultIndex = 1
        if separator == nil or separator == "" then
            for i = 1, #source do
                result[resultIndex] = sub(source, i, i)
                resultIndex = resultIndex + 1
            end
        else
            local currentPos = 1
            while resultIndex <= limit do
                local startPos, endPos = find(source, separator, currentPos, true)
                if not startPos then
                    break
                end
                result[resultIndex] = sub(source, currentPos, startPos - 1)
                resultIndex = resultIndex + 1
                currentPos = endPos + 1
            end
            if resultIndex <= limit then
                result[resultIndex] = sub(source, currentPos)
            end
        end
        return result
    end
end

local __TS__StringReplaceAll
do
    local sub = string.sub
    local find = string.find
    function __TS__StringReplaceAll(source, searchValue, replaceValue)
        if type(replaceValue) == "string" then
            local concat = table.concat(
                __TS__StringSplit(source, searchValue),
                replaceValue
            )
            if #searchValue == 0 then
                return (replaceValue .. concat) .. replaceValue
            end
            return concat
        end
        local parts = {}
        local partsIndex = 1
        if #searchValue == 0 then
            parts[1] = replaceValue(nil, "", 0, source)
            partsIndex = 2
            for i = 1, #source do
                parts[partsIndex] = sub(source, i, i)
                parts[partsIndex + 1] = replaceValue(nil, "", i, source)
                partsIndex = partsIndex + 2
            end
        else
            local currentPos = 1
            while true do
                local startPos, endPos = find(source, searchValue, currentPos, true)
                if not startPos then
                    break
                end
                parts[partsIndex] = sub(source, currentPos, startPos - 1)
                parts[partsIndex + 1] = replaceValue(nil, searchValue, startPos - 1, source)
                partsIndex = partsIndex + 2
                currentPos = endPos + 1
            end
            parts[partsIndex] = sub(source, currentPos)
        end
        return table.concat(parts)
    end
end

local function __TS__StringSlice(self, start, ____end)
    if start == nil or start ~= start then
        start = 0
    end
    if ____end ~= ____end then
        ____end = 0
    end
    if start >= 0 then
        start = start + 1
    end
    if ____end ~= nil and ____end < 0 then
        ____end = ____end - 1
    end
    return string.sub(self, start, ____end)
end

local function __TS__StringStartsWith(self, searchString, position)
    if position == nil or position < 0 then
        position = 0
    end
    return string.sub(self, position + 1, #searchString + position) == searchString
end

local function __TS__StringSubstr(self, from, length)
    if from ~= from then
        from = 0
    end
    if length ~= nil then
        if length ~= length or length <= 0 then
            return ""
        end
        length = length + from
    end
    if from >= 0 then
        from = from + 1
    end
    return string.sub(self, from, length)
end

local function __TS__StringTrim(self)
    local result = string.gsub(self, "^[%s ﻿]*(.-)[%s ﻿]*$", "%1")
    return result
end

local function __TS__StringTrimEnd(self)
    local result = string.gsub(self, "[%s ﻿]*$", "")
    return result
end

local function __TS__StringTrimStart(self)
    local result = string.gsub(self, "^[%s ﻿]*", "")
    return result
end

local __TS__SymbolRegistryFor, __TS__SymbolRegistryKeyFor
do
    local symbolRegistry = {}
    function __TS__SymbolRegistryFor(key)
        if not symbolRegistry[key] then
            symbolRegistry[key] = __TS__Symbol(key)
        end
        return symbolRegistry[key]
    end
    function __TS__SymbolRegistryKeyFor(sym)
        for key in pairs(symbolRegistry) do
            if symbolRegistry[key] == sym then
                return key
            end
        end
        return nil
    end
end

local function __TS__TypeOf(value)
    local luaType = type(value)
    if luaType == "table" then
        return "object"
    elseif luaType == "nil" then
        return "undefined"
    else
        return luaType
    end
end

local function __TS__Using(self, cb, ...)
    local args = {...}
    local thrownError
    local ok, result = xpcall(
        function() return cb(
            nil,
            __TS__Unpack(args)
        ) end,
        function(err)
            thrownError = err
            return thrownError
        end
    )
    local argArray = {__TS__Unpack(args)}
    do
        local i = #argArray - 1
        while i >= 0 do
            local ____self_0 = argArray[i + 1]
            ____self_0[Symbol.dispose](____self_0)
            i = i - 1
        end
    end
    if not ok then
        error(thrownError, 0)
    end
    return result
end

local function __TS__UsingAsync(self, cb, ...)
    local args = {...}
    return __TS__AsyncAwaiter(function(____awaiter_resolve)
        local thrownError
        local ok, result = xpcall(
            function() return cb(
                nil,
                __TS__Unpack(args)
            ) end,
            function(err)
                thrownError = err
                return thrownError
            end
        )
        local argArray = {__TS__Unpack(args)}
        do
            local i = #argArray - 1
            while i >= 0 do
                if argArray[i + 1][Symbol.dispose] ~= nil then
                    local ____self_0 = argArray[i + 1]
                    ____self_0[Symbol.dispose](____self_0)
                end
                if argArray[i + 1][Symbol.asyncDispose] ~= nil then
                    local ____self_1 = argArray[i + 1]
                    __TS__Await(____self_1[Symbol.asyncDispose](____self_1))
                end
                i = i - 1
            end
        end
        if not ok then
            error(thrownError, 0)
        end
        return ____awaiter_resolve(nil, result)
    end)
end

return {
  __TS__ArrayAt = __TS__ArrayAt,
  __TS__ArrayConcat = __TS__ArrayConcat,
  __TS__ArrayEntries = __TS__ArrayEntries,
  __TS__ArrayEvery = __TS__ArrayEvery,
  __TS__ArrayFill = __TS__ArrayFill,
  __TS__ArrayFilter = __TS__ArrayFilter,
  __TS__ArrayForEach = __TS__ArrayForEach,
  __TS__ArrayFind = __TS__ArrayFind,
  __TS__ArrayFindIndex = __TS__ArrayFindIndex,
  __TS__ArrayFrom = __TS__ArrayFrom,
  __TS__ArrayIncludes = __TS__ArrayIncludes,
  __TS__ArrayIndexOf = __TS__ArrayIndexOf,
  __TS__ArrayIsArray = __TS__ArrayIsArray,
  __TS__ArrayJoin = __TS__ArrayJoin,
  __TS__ArrayMap = __TS__ArrayMap,
  __TS__ArrayPush = __TS__ArrayPush,
  __TS__ArrayPushArray = __TS__ArrayPushArray,
  __TS__ArrayReduce = __TS__ArrayReduce,
  __TS__ArrayReduceRight = __TS__ArrayReduceRight,
  __TS__ArrayReverse = __TS__ArrayReverse,
  __TS__ArrayUnshift = __TS__ArrayUnshift,
  __TS__ArraySort = __TS__ArraySort,
  __TS__ArraySlice = __TS__ArraySlice,
  __TS__ArraySome = __TS__ArraySome,
  __TS__ArraySplice = __TS__ArraySplice,
  __TS__ArrayToObject = __TS__ArrayToObject,
  __TS__ArrayFlat = __TS__ArrayFlat,
  __TS__ArrayFlatMap = __TS__ArrayFlatMap,
  __TS__ArraySetLength = __TS__ArraySetLength,
  __TS__ArrayToReversed = __TS__ArrayToReversed,
  __TS__ArrayToSorted = __TS__ArrayToSorted,
  __TS__ArrayToSpliced = __TS__ArrayToSpliced,
  __TS__ArrayWith = __TS__ArrayWith,
  __TS__AsyncAwaiter = __TS__AsyncAwaiter,
  __TS__Await = __TS__Await,
  __TS__Class = __TS__Class,
  __TS__ClassExtends = __TS__ClassExtends,
  __TS__CloneDescriptor = __TS__CloneDescriptor,
  __TS__CountVarargs = __TS__CountVarargs,
  __TS__Decorate = __TS__Decorate,
  __TS__DecorateLegacy = __TS__DecorateLegacy,
  __TS__DecorateParam = __TS__DecorateParam,
  __TS__Delete = __TS__Delete,
  __TS__DelegatedYield = __TS__DelegatedYield,
  __TS__DescriptorGet = __TS__DescriptorGet,
  __TS__DescriptorSet = __TS__DescriptorSet,
  Error = Error,
  RangeError = RangeError,
  ReferenceError = ReferenceError,
  SyntaxError = SyntaxError,
  TypeError = TypeError,
  URIError = URIError,
  __TS__FunctionBind = __TS__FunctionBind,
  __TS__Generator = __TS__Generator,
  __TS__InstanceOf = __TS__InstanceOf,
  __TS__InstanceOfObject = __TS__InstanceOfObject,
  __TS__Iterator = __TS__Iterator,
  __TS__LuaIteratorSpread = __TS__LuaIteratorSpread,
  Map = Map,
  __TS__MapGroupBy = __TS__MapGroupBy,
  __TS__Match = __TS__Match,
  __TS__MathAtan2 = __TS__MathAtan2,
  __TS__MathModf = __TS__MathModf,
  __TS__MathSign = __TS__MathSign,
  __TS__MathTrunc = __TS__MathTrunc,
  __TS__New = __TS__New,
  __TS__Number = __TS__Number,
  __TS__NumberIsFinite = __TS__NumberIsFinite,
  __TS__NumberIsInteger = __TS__NumberIsInteger,
  __TS__NumberIsNaN = __TS__NumberIsNaN,
  __TS__ParseInt = __TS__ParseInt,
  __TS__ParseFloat = __TS__ParseFloat,
  __TS__NumberToString = __TS__NumberToString,
  __TS__NumberToFixed = __TS__NumberToFixed,
  __TS__ObjectAssign = __TS__ObjectAssign,
  __TS__ObjectDefineProperty = __TS__ObjectDefineProperty,
  __TS__ObjectEntries = __TS__ObjectEntries,
  __TS__ObjectFromEntries = __TS__ObjectFromEntries,
  __TS__ObjectGetOwnPropertyDescriptor = __TS__ObjectGetOwnPropertyDescriptor,
  __TS__ObjectGetOwnPropertyDescriptors = __TS__ObjectGetOwnPropertyDescriptors,
  __TS__ObjectGroupBy = __TS__ObjectGroupBy,
  __TS__ObjectKeys = __TS__ObjectKeys,
  __TS__ObjectRest = __TS__ObjectRest,
  __TS__ObjectValues = __TS__ObjectValues,
  __TS__ParseFloat = __TS__ParseFloat,
  __TS__ParseInt = __TS__ParseInt,
  __TS__Promise = __TS__Promise,
  __TS__PromiseAll = __TS__PromiseAll,
  __TS__PromiseAllSettled = __TS__PromiseAllSettled,
  __TS__PromiseAny = __TS__PromiseAny,
  __TS__PromiseRace = __TS__PromiseRace,
  Set = Set,
  __TS__SetDescriptor = __TS__SetDescriptor,
  __TS__SparseArrayNew = __TS__SparseArrayNew,
  __TS__SparseArrayPush = __TS__SparseArrayPush,
  __TS__SparseArraySpread = __TS__SparseArraySpread,
  WeakMap = WeakMap,
  WeakSet = WeakSet,
  __TS__SourceMapTraceBack = __TS__SourceMapTraceBack,
  __TS__Spread = __TS__Spread,
  __TS__StringAccess = __TS__StringAccess,
  __TS__StringCharAt = __TS__StringCharAt,
  __TS__StringCharCodeAt = __TS__StringCharCodeAt,
  __TS__StringEndsWith = __TS__StringEndsWith,
  __TS__StringIncludes = __TS__StringIncludes,
  __TS__StringPadEnd = __TS__StringPadEnd,
  __TS__StringPadStart = __TS__StringPadStart,
  __TS__StringReplace = __TS__StringReplace,
  __TS__StringReplaceAll = __TS__StringReplaceAll,
  __TS__StringSlice = __TS__StringSlice,
  __TS__StringSplit = __TS__StringSplit,
  __TS__StringStartsWith = __TS__StringStartsWith,
  __TS__StringSubstr = __TS__StringSubstr,
  __TS__StringSubstring = __TS__StringSubstring,
  __TS__StringTrim = __TS__StringTrim,
  __TS__StringTrimEnd = __TS__StringTrimEnd,
  __TS__StringTrimStart = __TS__StringTrimStart,
  __TS__Symbol = __TS__Symbol,
  Symbol = Symbol,
  __TS__SymbolRegistryFor = __TS__SymbolRegistryFor,
  __TS__SymbolRegistryKeyFor = __TS__SymbolRegistryKeyFor,
  __TS__TypeOf = __TS__TypeOf,
  __TS__Unpack = __TS__Unpack,
  __TS__Using = __TS__Using,
  __TS__UsingAsync = __TS__UsingAsync
}
 end,
["src.core.math.scalar"] = function(...) 
--[[ Generated with https://github.com/TypeScriptToLua/TypeScriptToLua ]]
local ____exports = {}
--- Scalar helpers. Everything in src/core must behave identically when compiled
-- to JavaScript and to Lua (via TypeScriptToLua), so modulo and rounding go
-- through these helpers instead of operators whose semantics differ.
____exports.PI = math.pi
____exports.TAU = math.pi * 2
____exports.DEG = math.pi / 180
____exports.G = 9.81
____exports.EPS = 1e-9
--- Floored modulo: result always has the sign of `b` (Lua semantics, also what we want for angles).
function ____exports.mod(a, b)
    return a - math.floor(a / b) * b
end
function ____exports.clamp(v, lo, hi)
    return v < lo and lo or (v > hi and hi or v)
end
function ____exports.clamp01(v)
    return v < 0 and 0 or (v > 1 and 1 or v)
end
function ____exports.lerp(a, b, t)
    return a + (b - a) * t
end
function ____exports.invLerp(a, b, v)
    return b == a and 0 or (v - a) / (b - a)
end
function ____exports.sign(v)
    return v > 0 and 1 or (v < 0 and -1 or 0)
end
--- Wrap angle to (-PI, PI].
function ____exports.wrapAngle(a)
    local w = ____exports.mod(a + ____exports.PI, ____exports.TAU) - ____exports.PI
    return w == -____exports.PI and ____exports.PI or w
end
--- Signed smallest difference b - a in (-PI, PI].
function ____exports.angleDiff(a, b)
    return ____exports.wrapAngle(b - a)
end
--- Move `current` toward `target` by at most `maxDelta`.
function ____exports.approach(current, target, maxDelta)
    if current < target then
        return math.min(current + maxDelta, target)
    end
    return math.max(current - maxDelta, target)
end
--- Frame-rate independent exponential smoothing factor.
function ____exports.smoothFactor(rate, dt)
    return 1 - math.exp(-rate * dt)
end
function ____exports.smoothstep(e0, e1, x)
    local t = ____exports.clamp01((x - e0) / (e1 - e0))
    return t * t * (3 - 2 * t)
end
function ____exports.round(v, decimals)
    local f = 10 ^ decimals
    return math.floor(v * f + 0.5) / f
end
function ____exports.hypot(x, y)
    return math.sqrt(x * x + y * y)
end
--- Piecewise-linear lookup in a table of [x, y] pairs sorted by x.
function ____exports.curve(points, x)
    local n = #points
    if x <= points[1][1] then
        return points[1][2]
    end
    if x >= points[n][1] then
        return points[n][2]
    end
    do
        local i = 1
        while i < n do
            local p1 = points[i + 1]
            if x <= p1[1] then
                local p0 = points[i]
                return ____exports.lerp(p0[2], p1[2], (x - p0[1]) / (p1[1] - p0[1]))
            end
            i = i + 1
        end
    end
    return points[n][2]
end
--- km/h <-> m/s
____exports.KMH = 1 / 3.6
function ____exports.toKmh(ms)
    return ms * 3.6
end
return ____exports
 end,
["src.core.math.rng"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local __TS__New = ____lualib.__TS__New
local ____exports = {}
--- Deterministic pseudo random generator (Park–Miller minimal standard).
-- Uses only double arithmetic that is exact below 2^53, so JS and Lua produce
-- the same sequence for the same seed.
____exports.Rng = __TS__Class()
local Rng = ____exports.Rng
Rng.name = "Rng"
function Rng.prototype.____constructor(self, seed)
    local s = math.floor(math.abs(seed)) - math.floor(math.abs(seed) / 2147483646) * 2147483646
    if s <= 0 then
        s = s + 2147483646
    end
    self.state = s
end
function Rng.prototype.next(self)
    local prod = self.state * 16807
    self.state = prod - math.floor(prod / 2147483647) * 2147483647
    return (self.state - 1) / 2147483646
end
function Rng.prototype.range(self, lo, hi)
    return lo + (hi - lo) * self:next()
end
function Rng.prototype.int(self, lo, hi)
    return lo + math.floor(self:next() * (hi - lo + 1))
end
function Rng.prototype.chance(self, p)
    return self:next() < p
end
function Rng.prototype.pick(self, items)
    return items[math.floor(self:next() * #items) + 1]
end
function Rng.prototype.gauss(self, mean, sigma)
    local u = self:next() + self:next() + self:next() + self:next() - 2
    return mean + sigma * u * 1.7320508
end
function Rng.prototype.weighted(self, entries)
    local total = 0
    for ____, e in ipairs(entries) do
        total = total + e.weight
    end
    local r = self:next() * total
    for ____, e in ipairs(entries) do
        r = r - e.weight
        if r <= 0 then
            return e.item
        end
    end
    return entries[#entries].item
end
function Rng.prototype.fork(self)
    return __TS__New(
        ____exports.Rng,
        math.floor(self:next() * 2147483000) + 1
    )
end
return ____exports
 end,
["src.core.math.vec"] = function(...) 
--[[ Generated with https://github.com/TypeScriptToLua/TypeScriptToLua ]]
local ____exports = {}
function ____exports.v2(x, y)
    return {x = x, y = y}
end
function ____exports.add(a, b)
    return {x = a.x + b.x, y = a.y + b.y}
end
function ____exports.sub(a, b)
    return {x = a.x - b.x, y = a.y - b.y}
end
function ____exports.scale(a, s)
    return {x = a.x * s, y = a.y * s}
end
function ____exports.dot(a, b)
    return a.x * b.x + a.y * b.y
end
function ____exports.cross(a, b)
    return a.x * b.y - a.y * b.x
end
function ____exports.len(a)
    return math.sqrt(a.x * a.x + a.y * a.y)
end
function ____exports.dist(a, b)
    local dx = a.x - b.x
    local dy = a.y - b.y
    return math.sqrt(dx * dx + dy * dy)
end
function ____exports.dist2(ax, ay, bx, by)
    local dx = ax - bx
    local dy = ay - by
    return dx * dx + dy * dy
end
function ____exports.norm(a)
    local l = ____exports.len(a)
    return l > 1e-12 and ({x = a.x / l, y = a.y / l}) or ({x = 0, y = 0})
end
--- Left-hand perpendicular (rotate +90 degrees).
function ____exports.perp(a)
    return {x = -a.y, y = a.x}
end
function ____exports.fromAngle(theta)
    return {
        x = math.cos(theta),
        y = math.sin(theta)
    }
end
function ____exports.angleOf(a)
    return math.atan2(a.y, a.x)
end
function ____exports.lerpV(a, b, t)
    return {x = a.x + (b.x - a.x) * t, y = a.y + (b.y - a.y) * t}
end
return ____exports
 end,
["src.core.math.path"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local ____exports = {}
--- A polyline parameterised by arc length. All lane and connector geometry is
-- stored this way: sampling is a binary search plus a lerp, cheap enough for
-- hundreds of vehicles per frame.
____exports.Path = __TS__Class()
local Path = ____exports.Path
Path.name = "Path"
function Path.prototype.____constructor(self, points)
    self.xs = {}
    self.ys = {}
    self.cum = {}
    local total = 0
    do
        local i = 0
        while i < #points do
            do
                local __continue3
                repeat
                    local p = points[i + 1]
                    if i > 0 then
                        local dx = p.x - self.xs[#self.xs]
                        local dy = p.y - self.ys[#self.ys]
                        local d = math.sqrt(dx * dx + dy * dy)
                        if d < 0.000001 then
                            __continue3 = true
                            break
                        end
                        total = total + d
                    end
                    local ____self_xs_0 = self.xs
                    ____self_xs_0[#____self_xs_0 + 1] = p.x
                    local ____self_ys_1 = self.ys
                    ____self_ys_1[#____self_ys_1 + 1] = p.y
                    local ____self_cum_2 = self.cum
                    ____self_cum_2[#____self_cum_2 + 1] = total
                    __continue3 = true
                until true
                if not __continue3 then
                    break
                end
            end
            i = i + 1
        end
    end
    if #self.xs == 1 then
        local ____self_xs_3 = self.xs
        ____self_xs_3[#____self_xs_3 + 1] = self.xs[1] + 0.001
        local ____self_ys_4 = self.ys
        ____self_ys_4[#____self_ys_4 + 1] = self.ys[1]
        local ____self_cum_5 = self.cum
        ____self_cum_5[#____self_cum_5 + 1] = 0.001
        total = 0.001
    end
    self.length = total
end
function Path.prototype.seg(self, s)
    local cum = self.cum
    local lo = 0
    local hi = #cum - 2
    if s <= 0 then
        return 0
    end
    if s >= self.length then
        return hi
    end
    while lo < hi do
        local mid = math.floor((lo + hi + 1) / 2)
        if cum[mid + 1] <= s then
            lo = mid
        else
            hi = mid - 1
        end
    end
    return lo
end
function Path.prototype.sample(self, s, out)
    local i = self:seg(s)
    local x0 = self.xs[i + 1]
    local y0 = self.ys[i + 1]
    local x1 = self.xs[i + 1 + 1]
    local y1 = self.ys[i + 1 + 1]
    local segLen = self.cum[i + 1 + 1] - self.cum[i + 1]
    local t = segLen > 0 and (s - self.cum[i + 1]) / segLen or 0
    if i == 0 and t < 0 then
        t = (s - self.cum[1]) / segLen
    end
    out.x = x0 + (x1 - x0) * t
    out.y = y0 + (y1 - y0) * t
    out.heading = math.atan2(y1 - y0, x1 - x0)
    return out
end
function Path.prototype.headingAt(self, s)
    local i = self:seg(s)
    return math.atan2(self.ys[i + 1 + 1] - self.ys[i + 1], self.xs[i + 1 + 1] - self.xs[i + 1])
end
function Path.prototype.curvatureAt(self, s, window)
    local a = self:headingAt(s - window)
    local b = self:headingAt(s + window)
    local d = b - a
    while d > math.pi do
        d = d - math.pi * 2
    end
    while d < -math.pi do
        d = d + math.pi * 2
    end
    return d / (2 * window)
end
function Path.prototype.project(self, px, py, out)
    local best = math.huge
    local bestS = 0
    local bestLat = 0
    local n = #self.xs
    do
        local i = 0
        while i < n - 1 do
            local x0 = self.xs[i + 1]
            local y0 = self.ys[i + 1]
            local dx = self.xs[i + 1 + 1] - x0
            local dy = self.ys[i + 1 + 1] - y0
            local l2 = dx * dx + dy * dy
            local t = l2 > 0 and ((px - x0) * dx + (py - y0) * dy) / l2 or 0
            if t < 0 then
                t = 0
            elseif t > 1 then
                t = 1
            end
            local cx = x0 + dx * t
            local cy = y0 + dy * t
            local ex = px - cx
            local ey = py - cy
            local d2 = ex * ex + ey * ey
            if d2 < best then
                best = d2
                bestS = self.cum[i + 1] + t * (self.cum[i + 1 + 1] - self.cum[i + 1])
                local l = math.sqrt(l2)
                bestLat = l > 0 and (dx * ey - dy * ex) / l or 0
            end
            i = i + 1
        end
    end
    out.s = bestS
    out.lateral = bestLat
    out.d2 = best
    return out
end
function Path.prototype.points(self)
    local r = {}
    do
        local i = 0
        while i < #self.xs do
            r[#r + 1] = {x = self.xs[i + 1], y = self.ys[i + 1]}
            i = i + 1
        end
    end
    return r
end
function Path.prototype.slice(self, s0, s1)
    local tmp = {x = 0, y = 0, heading = 0}
    local r = {}
    self:sample(s0, tmp)
    r[#r + 1] = {x = tmp.x, y = tmp.y}
    do
        local i = 0
        while i < #self.cum do
            if self.cum[i + 1] > s0 and self.cum[i + 1] < s1 then
                r[#r + 1] = {x = self.xs[i + 1], y = self.ys[i + 1]}
            end
            i = i + 1
        end
    end
    self:sample(s1, tmp)
    r[#r + 1] = {x = tmp.x, y = tmp.y}
    return r
end
--- Offset a polyline sideways by `offset` metres (positive = left).
function ____exports.offsetPolyline(points, offset)
    local n = #points
    local out = {}
    do
        local i = 0
        while i < n do
            local nx = 0
            local ny = 0
            if i > 0 then
                local dx = points[i + 1].x - points[i].x
                local dy = points[i + 1].y - points[i].y
                local l = math.sqrt(dx * dx + dy * dy)
                if l > 0 then
                    nx = nx + -dy / l
                    ny = ny + dx / l
                end
            end
            if i < n - 1 then
                local dx = points[i + 1 + 1].x - points[i + 1].x
                local dy = points[i + 1 + 1].y - points[i + 1].y
                local l = math.sqrt(dx * dx + dy * dy)
                if l > 0 then
                    nx = nx + -dy / l
                    ny = ny + dx / l
                end
            end
            local nl = math.sqrt(nx * nx + ny * ny)
            if nl > 0 then
                nx = nx / nl
                ny = ny / nl
            end
            local k = 1
            if i > 0 and i < n - 1 then
                local dx = points[i + 1 + 1].x - points[i + 1].x
                local dy = points[i + 1 + 1].y - points[i + 1].y
                local l = math.sqrt(dx * dx + dy * dy)
                if l > 0 then
                    local c = (nx * -dy + ny * dx) / l
                    if c > 0.3 then
                        k = 1 / c
                    end
                end
            end
            out[#out + 1] = {x = points[i + 1].x + nx * offset * k, y = points[i + 1].y + ny * offset * k}
            i = i + 1
        end
    end
    return out
end
--- Cubic Bezier flattened to a polyline.
function ____exports.cubicBezier(p0, p1, p2, p3, steps)
    local out = {}
    do
        local i = 0
        while i <= steps do
            local t = i / steps
            local u = 1 - t
            local a = u * u * u
            local b = 3 * u * u * t
            local c = 3 * u * t * t
            local d = t * t * t
            out[#out + 1] = {x = a * p0.x + b * p1.x + c * p2.x + d * p3.x, y = a * p0.y + b * p1.y + c * p2.y + d * p3.y}
            i = i + 1
        end
    end
    return out
end
--- Intersection of segments p1-p2 and p3-p4. Returns parameters (t on first,
-- u on second) or undefined when they don't cross.
function ____exports.segmentIntersect(x1, y1, x2, y2, x3, y3, x4, y4)
    local d = (x2 - x1) * (y4 - y3) - (y2 - y1) * (x4 - x3)
    if math.abs(d) < 1e-12 then
        return nil
    end
    local t = ((x3 - x1) * (y4 - y3) - (y3 - y1) * (x4 - x3)) / d
    local u = ((x3 - x1) * (y2 - y1) - (y3 - y1) * (x2 - x1)) / d
    if t < 0 or t > 1 or u < 0 or u > 1 then
        return nil
    end
    return {t = t, u = u}
end
--- First crossing between two paths, as arc lengths on each, or undefined.
-- `skipStart`/`skipEnd` ignore crossings near the shared start/end points of
-- paths that merge or diverge (those are handled as merges, not crossings).
function ____exports.pathCrossing(a, b)
    local best = nil
    do
        local i = 0
        while i < #a.xs - 1 do
            do
                local j = 0
                while j < #b.xs - 1 do
                    local hit = ____exports.segmentIntersect(
                        a.xs[i + 1],
                        a.ys[i + 1],
                        a.xs[i + 1 + 1],
                        a.ys[i + 1 + 1],
                        b.xs[j + 1],
                        b.ys[j + 1],
                        b.xs[j + 1 + 1],
                        b.ys[j + 1 + 1]
                    )
                    if hit ~= nil then
                        local sa = a.cum[i + 1] + hit.t * (a.cum[i + 1 + 1] - a.cum[i + 1])
                        local sb = b.cum[j + 1] + hit.u * (b.cum[j + 1 + 1] - b.cum[j + 1])
                        if best == nil or sa < best.sa then
                            best = {sa = sa, sb = sb}
                        end
                    end
                    j = j + 1
                end
            end
            i = i + 1
        end
    end
    return best
end
--- Minimum distance between two paths with the arc lengths where it occurs (sampled).
function ____exports.pathClosest(a, b, step)
    local tmp = {s = 0, lateral = 0, d2 = 0}
    local ps = {x = 0, y = 0, heading = 0}
    local best = {sa = 0, sb = 0, d = math.huge}
    local n = math.max(
        2,
        math.ceil(a.length / step)
    )
    do
        local i = 0
        while i <= n do
            local s = a.length * i / n
            a:sample(s, ps)
            b:project(ps.x, ps.y, tmp)
            local d = math.sqrt(tmp.d2)
            if d < best.d then
                best = {sa = s, sb = tmp.s, d = d}
            end
            i = i + 1
        end
    end
    return best
end
return ____exports
 end,
["src.core.util.spatialHash"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local Map = ____lualib.Map
local __TS__New = ____lualib.__TS__New
local __TS__ArrayIndexOf = ____lualib.__TS__ArrayIndexOf
local ____exports = {}
--- Uniform grid spatial hash for broad-phase proximity queries. Cell keys are
-- numbers (not strings) to keep the Lua build fast.
____exports.SpatialHash = __TS__Class()
local SpatialHash = ____exports.SpatialHash
SpatialHash.name = "SpatialHash"
function SpatialHash.prototype.____constructor(self, cellSize)
    self.cellSize = cellSize
    self.cells = __TS__New(Map)
    self.inv = 1 / cellSize
end
function SpatialHash.prototype.key(self, cx, cy)
    return (cx + 32768) * 65536 + (cy + 32768)
end
function SpatialHash.prototype.clear(self)
    self.cells = __TS__New(Map)
end
function SpatialHash.prototype.insert(self, item, x, y, radius)
    local x0 = math.floor((x - radius) * self.inv)
    local x1 = math.floor((x + radius) * self.inv)
    local y0 = math.floor((y - radius) * self.inv)
    local y1 = math.floor((y + radius) * self.inv)
    do
        local cx = x0
        while cx <= x1 do
            do
                local cy = y0
                while cy <= y1 do
                    local k = self:key(cx, cy)
                    local cell = self.cells:get(k)
                    if cell == nil then
                        cell = {}
                        self.cells:set(k, cell)
                    end
                    cell[#cell + 1] = item
                    cy = cy + 1
                end
            end
            cx = cx + 1
        end
    end
end
function SpatialHash.prototype.query(self, x, y, radius, out)
    local x0 = math.floor((x - radius) * self.inv)
    local x1 = math.floor((x + radius) * self.inv)
    local y0 = math.floor((y - radius) * self.inv)
    local y1 = math.floor((y + radius) * self.inv)
    do
        local cx = x0
        while cx <= x1 do
            do
                local cy = y0
                while cy <= y1 do
                    local cell = self.cells:get(self:key(cx, cy))
                    if cell ~= nil then
                        do
                            local i = 0
                            while i < #cell do
                                local it = cell[i + 1]
                                if __TS__ArrayIndexOf(out, it) < 0 then
                                    out[#out + 1] = it
                                end
                                i = i + 1
                            end
                        end
                    end
                    cy = cy + 1
                end
            end
            cx = cx + 1
        end
    end
    return out
end
return ____exports
 end,
["src.core.road.types"] = function(...) 
--[[ Generated with https://github.com/TypeScriptToLua/TypeScriptToLua ]]
local ____exports = {}
return ____exports
 end,
["src.core.road.network"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local __TS__New = ____lualib.__TS__New
local __TS__ArrayIndexOf = ____lualib.__TS__ArrayIndexOf
local ____exports = {}
local ____path = require("src.core.math.path")
local Path = ____path.Path
local cubicBezier = ____path.cubicBezier
local offsetPolyline = ____path.offsetPolyline
local ____scalar = require("src.core.math.scalar")
local angleDiff = ____scalar.angleDiff
local clamp = ____scalar.clamp
local DEG = ____scalar.DEG
local KMH = ____scalar.KMH
local ____spatialHash = require("src.core.util.spatialHash")
local SpatialHash = ____spatialHash.SpatialHash
____exports.Node = __TS__Class()
local Node = ____exports.Node
Node.name = "Node"
function Node.prototype.____constructor(self, id, x, y, control)
    self.id = id
    self.x = x
    self.y = y
    self.control = control
    self.roads = {}
    self.approaches = {}
    self.connectors = {}
    self.radius = 0
    self.signalId = -1
    self.redLightCamera = false
    self.name = ""
end
function Node.prototype.isTrivial(self)
    return #self.roads <= 2
end
____exports.Road = __TS__Class()
local Road = ____exports.Road
Road.name = "Road"
function Road.prototype.____constructor(self, id, a, b, opts)
    self.id = id
    self.a = a
    self.b = b
    self.opts = opts
    self.lanesAB = {}
    self.lanesBA = {}
    self.trimA = 0
    self.trimB = 0
    self.speedCamera = false
    local pts = {{x = a.x, y = a.y}}
    if opts.via ~= nil then
        for ____, p in ipairs(opts.via) do
            pts[#pts + 1] = {x = p.x, y = p.y}
        end
    end
    pts[#pts + 1] = {x = b.x, y = b.y}
    self.centerline = pts
    self.centerPath = __TS__New(Path, pts)
    self.laneWidth = opts.laneWidth or 3.5
    self.medianWidth = opts.medianWidth or (opts.lanesBack > 0 and opts.lanesFwd > 0 and 0.3 or 0)
    self.sidewalkWidth = opts.sidewalkWidth or 3
    self.speedLimit = opts.speedLimitKmh * KMH
    self.kind = opts.kind
    self.name = opts.name
    local n = opts.lanesFwd + opts.lanesBack
    local m = (opts.lanesBack == 0 or opts.lanesFwd == 0) and 0 or self.medianWidth
    self.width = n * self.laneWidth + m
end
function Road.prototype.lanesInto(self, n)
    return n == self.b and self.lanesAB or self.lanesBA
end
function Road.prototype.lanesOutOf(self, n)
    return n == self.a and self.lanesAB or self.lanesBA
end
function Road.prototype.other(self, n)
    return n == self.a and self.b or self.a
end
function Road.prototype.headingFrom(self, n)
    local pts = self.centerline
    if n == self.a then
        return math.atan2(pts[2].y - pts[1].y, pts[2].x - pts[1].x)
    end
    local k = #pts - 1
    return math.atan2(pts[k].y - pts[k + 1].y, pts[k].x - pts[k + 1].x)
end
____exports.Lane = __TS__Class()
local Lane = ____exports.Lane
Lane.name = "Lane"
function Lane.prototype.____constructor(self, id, road, forward, index, from, to, path, width)
    self.id = id
    self.road = road
    self.forward = forward
    self.index = index
    self.from = from
    self.to = to
    self.path = path
    self.width = width
    self.kind = "lane"
    self.outgoing = {}
    self.incoming = {}
    self.left = nil
    self.right = nil
    self.approach = nil
    self.turns = {}
    self.occ = {}
    self.occCount = 0
    self.speedLimit = road.speedLimit
    self.length = path.length
end
function Lane.prototype.siblings(self)
    return self.forward and self.road.lanesAB or self.road.lanesBA
end
function Lane.prototype.hasTurn(self, t)
    return __TS__ArrayIndexOf(self.turns, t) >= 0
end
function Lane.prototype.connectorTo(self, road)
    for ____, c in ipairs(self.outgoing) do
        if c.to.road == road then
            return c
        end
    end
    return nil
end
____exports.Connector = __TS__Class()
local Connector = ____exports.Connector
Connector.name = "Connector"
function Connector.prototype.____constructor(self, id, node, from, to, turn, path)
    self.id = id
    self.node = node
    self.from = from
    self.to = to
    self.turn = turn
    self.path = path
    self.kind = "connector"
    self.conflicts = {}
    self.siblings = {}
    self.siblingDiverge = {}
    self.occ = {}
    self.occCount = 0
    self.length = path.length
    self.approach = from.approach
    local maxCurv = 0
    local steps = math.max(
        2,
        math.floor(path.length / 2)
    )
    do
        local i = 1
        while i < steps do
            local c = math.abs(path:curvatureAt(path.length * i / steps, 1.5))
            if c > maxCurv then
                maxCurv = c
            end
            i = i + 1
        end
    end
    local turnSpeed = maxCurv > 0.0001 and math.sqrt(2.6 / maxCurv) or 99
    self.speedLimit = math.min(
        math.max(from.speedLimit, to.speedLimit),
        math.max(3.5, turnSpeed)
    )
end
--- The set of lanes entering a node from one road. Signals and signs attach here.
____exports.Approach = __TS__Class()
local Approach = ____exports.Approach
Approach.name = "Approach"
function Approach.prototype.____constructor(self, id, node, road)
    self.id = id
    self.node = node
    self.road = road
    self.lanes = {}
    self.sign = "none"
    self.major = false
    self.group = -1
    self.stopA = {x = 0, y = 0}
    self.stopB = {x = 0, y = 0}
    self.heading = 0
end
local scratch = {x = 0, y = 0, heading = 0}
--- The drivable network: nodes, roads, lanes and intersection connectors.
-- Build with addNode/addRoad, then call finalize() once.
____exports.Network = __TS__Class()
local Network = ____exports.Network
Network.name = "Network"
function Network.prototype.____constructor(self)
    self.nodes = {}
    self.roads = {}
    self.lanes = {}
    self.connectors = {}
    self.approaches = {}
    self.segments = {}
    self.laneHash = __TS__New(SpatialHash, 40)
    self.connectorHash = __TS__New(SpatialHash, 40)
    self.finalized = false
    self.minX = math.huge
    self.minY = math.huge
    self.maxX = -math.huge
    self.maxY = -math.huge
end
function Network.prototype.addNode(self, x, y, control)
    local n = __TS__New(
        ____exports.Node,
        #self.nodes,
        x,
        y,
        control
    )
    local ____self_nodes_0 = self.nodes
    ____self_nodes_0[#____self_nodes_0 + 1] = n
    return n
end
function Network.prototype.addRoad(self, a, b, opts)
    local r = __TS__New(
        ____exports.Road,
        #self.roads,
        a,
        b,
        opts
    )
    local ____self_roads_1 = self.roads
    ____self_roads_1[#____self_roads_1 + 1] = r
    local ____a_roads_2 = a.roads
    ____a_roads_2[#____a_roads_2 + 1] = r
    local ____b_roads_3 = b.roads
    ____b_roads_3[#____b_roads_3 + 1] = r
    return r
end
function Network.prototype.finalize(self)
    if self.finalized then
        return
    end
    self:computeNodeRadii()
    for ____, r in ipairs(self.roads) do
        self:buildLanes(r)
    end
    for ____, n in ipairs(self.nodes) do
        self:buildApproaches(n)
    end
    for ____, n in ipairs(self.nodes) do
        self:buildConnectors(n)
    end
    for ____, n in ipairs(self.nodes) do
        self:buildConflicts(n)
    end
    for ____, l in ipairs(self.lanes) do
        local pts = l.path:points()
        do
            local i = 0
            while i < #pts do
                local p = pts[i + 1]
                self.minX = math.min(self.minX, p.x)
                self.minY = math.min(self.minY, p.y)
                self.maxX = math.max(self.maxX, p.x)
                self.maxY = math.max(self.maxY, p.y)
                i = i + 1
            end
        end
        self:hashPath(l.path, l, self.laneHash)
    end
    for ____, c in ipairs(self.connectors) do
        self:hashPath(c.path, c, self.connectorHash)
    end
    self.finalized = true
end
function Network.prototype.hashPath(self, path, item, hash)
    local step = 20
    local n = math.max(
        1,
        math.ceil(path.length / step)
    )
    do
        local i = 0
        while i <= n do
            path:sample(path.length * i / n, scratch)
            hash:insert(item, scratch.x, scratch.y, 12)
            i = i + 1
        end
    end
end
function Network.prototype.computeNodeRadii(self)
    for ____, n in ipairs(self.nodes) do
        local maxHalf = 0
        for ____, r in ipairs(n.roads) do
            maxHalf = math.max(maxHalf, r.width / 2)
        end
        if #n.roads <= 2 then
            n.radius = #n.roads == 2 and math.max(4, maxHalf * 0.9) or 2
        else
            n.radius = maxHalf + 4.5
        end
    end
    for ____, r in ipairs(self.roads) do
        r.trimA = r.a.radius
        r.trimB = r.b.radius
        local maxTrim = r.centerPath.length * 0.45
        if r.trimA > maxTrim then
            r.trimA = maxTrim
        end
        if r.trimB > maxTrim then
            r.trimB = maxTrim
        end
    end
end
function Network.prototype.buildLanes(self, r)
    local nF = r.opts.lanesFwd
    local nB = r.opts.lanesBack
    local w = r.laneWidth
    local oneWay = nF == 0 or nB == 0
    local halfMedian = oneWay and 0 or r.medianWidth / 2
    local fullLen = r.centerPath.length
    do
        local i = 0
        while i < nF do
            local off = oneWay and -(i + 0.5) * w + nF * w / 2 or -(halfMedian + (i + 0.5) * w)
            local pts = offsetPolyline(r.centerline, off)
            local full = __TS__New(Path, pts)
            local trimmed = __TS__New(
                Path,
                full:slice(r.trimA * (full.length / fullLen), full.length - r.trimB * (full.length / fullLen))
            )
            local lane = __TS__New(
                ____exports.Lane,
                #self.segments,
                r,
                true,
                i,
                r.a,
                r.b,
                trimmed,
                w
            )
            local ____self_segments_4 = self.segments
            ____self_segments_4[#____self_segments_4 + 1] = lane
            local ____self_lanes_5 = self.lanes
            ____self_lanes_5[#____self_lanes_5 + 1] = lane
            local ____r_lanesAB_6 = r.lanesAB
            ____r_lanesAB_6[#____r_lanesAB_6 + 1] = lane
            i = i + 1
        end
    end
    local rev = {}
    do
        local i = #r.centerline - 1
        while i >= 0 do
            rev[#rev + 1] = r.centerline[i + 1]
            i = i - 1
        end
    end
    do
        local i = 0
        while i < nB do
            local off = oneWay and -(i + 0.5) * w + nB * w / 2 or -(halfMedian + (i + 0.5) * w)
            local pts = offsetPolyline(rev, off)
            local full = __TS__New(Path, pts)
            local trimmed = __TS__New(
                Path,
                full:slice(r.trimB * (full.length / fullLen), full.length - r.trimA * (full.length / fullLen))
            )
            local lane = __TS__New(
                ____exports.Lane,
                #self.segments,
                r,
                false,
                i,
                r.b,
                r.a,
                trimmed,
                w
            )
            local ____self_segments_7 = self.segments
            ____self_segments_7[#____self_segments_7 + 1] = lane
            local ____self_lanes_8 = self.lanes
            ____self_lanes_8[#____self_lanes_8 + 1] = lane
            local ____r_lanesBA_9 = r.lanesBA
            ____r_lanesBA_9[#____r_lanesBA_9 + 1] = lane
            i = i + 1
        end
    end
    for ____, group in ipairs({r.lanesAB, r.lanesBA}) do
        do
            local i = 0
            while i < #group do
                group[i + 1].left = i > 0 and group[i] or nil
                group[i + 1].right = i < #group - 1 and group[i + 1 + 1] or nil
                i = i + 1
            end
        end
    end
end
function Network.prototype.buildApproaches(self, n)
    for ____, r in ipairs(n.roads) do
        do
            local __continue63
            repeat
                local lanes = r:lanesInto(n)
                if #lanes == 0 then
                    __continue63 = true
                    break
                end
                local ap = __TS__New(____exports.Approach, #self.approaches, n, r)
                local ____self_approaches_10 = self.approaches
                ____self_approaches_10[#____self_approaches_10 + 1] = ap
                local ____n_approaches_11 = n.approaches
                ____n_approaches_11[#____n_approaches_11 + 1] = ap
                for ____, l in ipairs(lanes) do
                    local ____ap_lanes_12 = ap.lanes
                    ____ap_lanes_12[#____ap_lanes_12 + 1] = l
                    l.approach = ap
                end
                local first = lanes[1]
                local last = lanes[#lanes]
                first.path:sample(first.length, scratch)
                ap.heading = scratch.heading
                local nx = -math.sin(ap.heading)
                local ny = math.cos(ap.heading)
                ap.stopB = {x = scratch.x + nx * first.width * 0.5, y = scratch.y + ny * first.width * 0.5}
                last.path:sample(last.length, scratch)
                ap.stopA = {x = scratch.x - nx * last.width * 0.5, y = scratch.y - ny * last.width * 0.5}
                __continue63 = true
            until true
            if not __continue63 then
                break
            end
        end
    end
end
function Network.prototype.turnBetween(self, inHeading, outHeading)
    local d = angleDiff(inHeading, outHeading)
    if math.abs(d) > 150 * DEG then
        return "uturn"
    end
    if d > 35 * DEG then
        return "left"
    end
    if d < -35 * DEG then
        return "right"
    end
    return "straight"
end
function Network.prototype.buildConnectors(self, n)
    for ____, ap in ipairs(n.approaches) do
        local inLanes = ap.lanes
        local k = #inLanes
        local outs = {}
        for ____, r in ipairs(n.roads) do
            do
                local __continue74
                repeat
                    if r == ap.road and #n.roads > 1 then
                        __continue74 = true
                        break
                    end
                    if #r:lanesOutOf(n) == 0 then
                        __continue74 = true
                        break
                    end
                    local t = self:turnBetween(
                        ap.heading,
                        r:headingFrom(n)
                    )
                    if t == "uturn" and #n.roads > 1 then
                        __continue74 = true
                        break
                    end
                    outs[#outs + 1] = {road = r, turn = t}
                    __continue74 = true
                until true
                if not __continue74 then
                    break
                end
            end
        end
        local hasL = false
        local hasS = false
        local hasR = false
        for ____, o in ipairs(outs) do
            if o.turn == "left" then
                hasL = true
            elseif o.turn == "straight" then
                hasS = true
            elseif o.turn == "right" then
                hasR = true
            end
        end
        do
            local i = 0
            while i < k do
                local t = inLanes[i + 1].turns
                if hasS then
                    local dedicatedLeft = k >= 3 and hasL
                    if not (dedicatedLeft and i == 0) then
                        t[#t + 1] = "straight"
                    end
                    if hasL and i == 0 then
                        t[#t + 1] = "left"
                    end
                    if hasR and i == k - 1 then
                        t[#t + 1] = "right"
                    end
                elseif hasL and hasR then
                    if k == 1 then
                        t[#t + 1] = "left"
                        t[#t + 1] = "right"
                    elseif i < math.ceil(k / 2) then
                        t[#t + 1] = "left"
                    else
                        t[#t + 1] = "right"
                    end
                elseif hasL then
                    t[#t + 1] = "left"
                elseif hasR then
                    t[#t + 1] = "right"
                end
                for ____, o in ipairs(outs) do
                    if o.turn == "uturn" then
                        t[#t + 1] = "uturn"
                    end
                end
                i = i + 1
            end
        end
        for ____, o in ipairs(outs) do
            local outLanes = o.road:lanesOutOf(n)
            local m = #outLanes
            local users = {}
            for ____, l in ipairs(inLanes) do
                if l:hasTurn(o.turn) then
                    users[#users + 1] = l
                end
            end
            do
                local u = 0
                while u < #users do
                    local from = users[u + 1]
                    local target
                    if o.turn == "left" or o.turn == "uturn" then
                        target = outLanes[clamp(u, 0, m - 1) + 1]
                    elseif o.turn == "right" then
                        target = outLanes[clamp(m - #users + u, 0, m - 1) + 1]
                    else
                        if m >= k then
                            target = outLanes[clamp(from.index, 0, m - 1) + 1]
                        else
                            target = outLanes[clamp(from.index - (k - m), 0, m - 1) + 1]
                        end
                    end
                    local c = self:makeConnector(n, from, target, o.turn)
                    local ____from_outgoing_13 = from.outgoing
                    ____from_outgoing_13[#____from_outgoing_13 + 1] = c
                    local ____target_incoming_14 = target.incoming
                    ____target_incoming_14[#____target_incoming_14 + 1] = c
                    local ____n_connectors_15 = n.connectors
                    ____n_connectors_15[#____n_connectors_15 + 1] = c
                    u = u + 1
                end
            end
        end
    end
    local tmp = {s = 0, lateral = 0, d2 = 0}
    for ____, c in ipairs(n.connectors) do
        for ____, d in ipairs(c.from.outgoing) do
            do
                local __continue111
                repeat
                    if d == c then
                        __continue111 = true
                        break
                    end
                    local ____c_siblings_16 = c.siblings
                    ____c_siblings_16[#____c_siblings_16 + 1] = d
                    local sep = c.length
                    local steps = math.max(
                        4,
                        math.ceil(c.length / 0.5)
                    )
                    do
                        local k = 1
                        while k <= steps do
                            local sk = c.length * k / steps
                            c.path:sample(sk, scratch)
                            d.path:project(scratch.x, scratch.y, tmp)
                            if tmp.d2 > 2.3 * 2.3 then
                                sep = sk
                                break
                            end
                            k = k + 1
                        end
                    end
                    local ____c_siblingDiverge_17 = c.siblingDiverge
                    ____c_siblingDiverge_17[#____c_siblingDiverge_17 + 1] = sep
                    __continue111 = true
                until true
                if not __continue111 then
                    break
                end
            end
        end
    end
end
function Network.prototype.makeConnector(self, n, from, to, turn)
    from.path:sample(from.length, scratch)
    local p0 = {x = scratch.x, y = scratch.y}
    local h0 = scratch.heading
    to.path:sample(0, scratch)
    local p3 = {x = scratch.x, y = scratch.y}
    local h3 = scratch.heading
    local dx = p3.x - p0.x
    local dy = p3.y - p0.y
    local d = math.sqrt(dx * dx + dy * dy)
    local pts
    if turn == "straight" and math.abs(angleDiff(h0, h3)) < 5 * DEG then
        pts = {p0, p3}
    else
        local k = turn == "uturn" and math.max(4, d * 1.2) or d * 0.45
        local p1 = {
            x = p0.x + math.cos(h0) * k,
            y = p0.y + math.sin(h0) * k
        }
        local p2 = {
            x = p3.x - math.cos(h3) * k,
            y = p3.y - math.sin(h3) * k
        }
        pts = cubicBezier(
            p0,
            p1,
            p2,
            p3,
            16
        )
    end
    local c = __TS__New(
        ____exports.Connector,
        #self.segments,
        n,
        from,
        to,
        turn,
        __TS__New(Path, pts)
    )
    local ____self_segments_18 = self.segments
    ____self_segments_18[#____self_segments_18 + 1] = c
    local ____self_connectors_19 = self.connectors
    ____self_connectors_19[#____self_connectors_19 + 1] = c
    return c
end
function Network.prototype.buildConflicts(self, n)
    local cs = n.connectors
    local tmp = {s = 0, lateral = 0, d2 = 0}
    do
        local i = 0
        while i < #cs do
            do
                local j = i + 1
                while j < #cs do
                    do
                        local __continue122
                        repeat
                            local a = cs[i + 1]
                            local b = cs[j + 1]
                            if a.from == b.from then
                                __continue122 = true
                                break
                            end
                            local merge = a.to == b.to
                            local clearance = 2.4
                            local s0 = -1
                            local s1 = -1
                            local os0 = math.huge
                            local os1 = -math.huge
                            local steps = math.max(
                                4,
                                math.ceil(a.length / 0.75)
                            )
                            do
                                local k = 0
                                while k <= steps do
                                    local s = a.length * k / steps
                                    a.path:sample(s, scratch)
                                    b.path:project(scratch.x, scratch.y, tmp)
                                    if tmp.d2 < clearance * clearance and tmp.s > 0.01 and tmp.s < b.length + 0.01 then
                                        if s0 < 0 then
                                            s0 = s
                                        end
                                        s1 = s
                                        if tmp.s < os0 then
                                            os0 = tmp.s
                                        end
                                        if tmp.s > os1 then
                                            os1 = tmp.s
                                        end
                                    end
                                    k = k + 1
                                end
                            end
                            if s0 < 0 then
                                __continue122 = true
                                break
                            end
                            local kind = merge and "merge" or "cross"
                            if merge then
                                s1 = a.length
                                os1 = b.length
                            end
                            local ____a_conflicts_20 = a.conflicts
                            ____a_conflicts_20[#____a_conflicts_20 + 1] = {
                                other = b,
                                s0 = math.max(0, s0 - 1),
                                s1 = math.min(a.length, s1 + 1),
                                os0 = math.max(0, os0 - 1),
                                os1 = math.min(b.length, os1 + 1),
                                kind = kind
                            }
                            local ____b_conflicts_21 = b.conflicts
                            ____b_conflicts_21[#____b_conflicts_21 + 1] = {
                                other = a,
                                s0 = math.max(0, os0 - 1),
                                s1 = math.min(b.length, os1 + 1),
                                os0 = math.max(0, s0 - 1),
                                os1 = math.min(a.length, s1 + 1),
                                kind = kind
                            }
                            __continue122 = true
                        until true
                        if not __continue122 then
                            break
                        end
                    end
                    j = j + 1
                end
            end
            i = i + 1
        end
    end
end
function Network.prototype.nearestLanes(self, x, y, radius, out)
    local cands = {}
    self.laneHash:query(x, y, radius, cands)
    local tmp = {s = 0, lateral = 0, d2 = 0}
    for ____, l in ipairs(cands) do
        l.path:project(x, y, tmp)
        local d = math.sqrt(tmp.d2)
        if d <= radius then
            out[#out + 1] = {lane = l, s = tmp.s, lateral = tmp.lateral, dist = d}
        end
    end
    do
        local i = 1
        while i < #out do
            local h = out[i + 1]
            local j = i - 1
            while j >= 0 and (out[j + 1].dist > h.dist or out[j + 1].dist == h.dist and out[j + 1].lane.id > h.lane.id) do
                out[j + 1 + 1] = out[j + 1]
                j = j - 1
            end
            out[j + 1 + 1] = h
            i = i + 1
        end
    end
    return out
end
function Network.prototype.nearestConnectors(self, x, y, radius, out)
    return self.connectorHash:query(x, y, radius, out)
end
function Network.prototype.nodeAt(self, x, y)
    local best = nil
    local bestD = math.huge
    for ____, n in ipairs(self.nodes) do
        do
            local __continue139
            repeat
                if #n.roads < 3 then
                    __continue139 = true
                    break
                end
                local dx = x - n.x
                local dy = y - n.y
                local d = dx * dx + dy * dy
                local r = n.radius - 1
                if d < r * r and d < bestD then
                    bestD = d
                    best = n
                end
                __continue139 = true
            until true
            if not __continue139 then
                break
            end
        end
    end
    return best
end
function Network.prototype.clearOccupancy(self)
    local segs = self.segments
    do
        local i = 0
        while i < #segs do
            segs[i + 1].occCount = 0
            i = i + 1
        end
    end
end
--- Append an occupancy entry to a segment, reusing pooled records.
function ____exports.addOccupant(seg, occ, s, v, phantom, lat)
    local i = seg.occCount
    if i < #seg.occ then
        local e = seg.occ[i + 1]
        e.occ = occ
        e.s = s
        e.v = v
        e.phantom = phantom
        e.lat = lat
    else
        local ____seg_occ_22 = seg.occ
        ____seg_occ_22[#____seg_occ_22 + 1] = {
            occ = occ,
            s = s,
            v = v,
            phantom = phantom,
            lat = lat
        }
    end
    seg.occCount = i + 1
end
return ____exports
 end,
["src.core.road.router"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local Map = ____lualib.Map
local __TS__New = ____lualib.__TS__New
local __TS__ArrayReverse = ____lualib.__TS__ArrayReverse
local ____exports = {}
--- Road-level A* with turn penalties and optional congestion costs.
-- Routes are lists of road steps after the vehicle's current lane.
____exports.Router = __TS__Class()
local Router = ____exports.Router
Router.name = "Router"
function Router.prototype.____constructor(self, net)
    self.net = net
    self.congestion = __TS__New(Map)
    self.maxSpeed = 1
    for ____, r in ipairs(net.roads) do
        self.maxSpeed = math.max(self.maxSpeed, r.speedLimit)
    end
end
function Router.prototype.controlDelay(self, n)
    repeat
        local ____switch6 = n.control
        local ____cond6 = ____switch6 == "signal"
        if ____cond6 then
            return 12
        end
        ____cond6 = ____cond6 or ____switch6 == "allway_stop"
        if ____cond6 then
            return 7
        end
        ____cond6 = ____cond6 or ____switch6 == "twoway_stop"
        if ____cond6 then
            return 4
        end
        do
            return 0.5
        end
    until true
end
function Router.prototype.route(self, startLane, goalRoad, goalForward)
    local goalNodeFrom = goalForward and goalRoad.a or goalRoad.b
    local goalTo = goalForward and goalRoad.b or goalRoad.a
    local start = startLane.to
    local open = {}
    local best = __TS__New(Map)
    local function h(n)
        return math.sqrt((n.x - goalNodeFrom.x) * (n.x - goalNodeFrom.x) + (n.y - goalNodeFrom.y) * (n.y - goalNodeFrom.y)) / self.maxSpeed
    end
    open[#open + 1] = {
        f = h(start),
        g = 0,
        node = start,
        via = startLane.road,
        parent = nil,
        stepRoad = nil,
        stepForward = true
    }
    local function key(n, via)
        return n.id * 100000 + (via == nil and 99999 or via.id)
    end
    local iterations = 0
    while #open > 0 and iterations < 20000 do
        iterations = iterations + 1
        local bi = 0
        do
            local i = 1
            while i < #open do
                local a = open[i + 1]
                local b = open[bi + 1]
                if a.f < b.f or a.f == b.f and a.node.id < b.node.id then
                    bi = i
                end
                i = i + 1
            end
        end
        local cur = open[bi + 1]
        open[bi + 1] = open[#open]
        table.remove(open)
        if cur.node == goalNodeFrom and cur.via ~= goalRoad then
            if self:canTurn(cur.node, cur.via, goalRoad, goalForward) then
                local steps = {{road = goalRoad, forward = goalForward, to = goalTo}}
                local e = cur
                while e ~= nil and e.stepRoad ~= nil do
                    steps[#steps + 1] = {road = e.stepRoad, forward = e.stepForward, to = e.node}
                    e = e.parent
                end
                __TS__ArrayReverse(steps)
                return steps
            end
        end
        for ____, r in ipairs(cur.node.roads) do
            do
                local __continue16
                repeat
                    if r == cur.via and #cur.node.roads > 1 then
                        __continue16 = true
                        break
                    end
                    local fwd = r.a == cur.node
                    if not self:canTurn(cur.node, cur.via, r, fwd) then
                        __continue16 = true
                        break
                    end
                    local next = r:other(cur.node)
                    local lanes = fwd and r.lanesAB or r.lanesBA
                    local cost = r.centerPath.length / r.speedLimit + self:controlDelay(next) + (self.congestion:get(r.id) or 0) + self:turnCost(cur.node, cur.via, r) + 3 / #lanes
                    local g = cur.g + cost
                    local k = key(next, r)
                    local prev = best:get(k)
                    if prev ~= nil and prev <= g then
                        __continue16 = true
                        break
                    end
                    best:set(k, g)
                    open[#open + 1] = {
                        f = g + h(next),
                        g = g,
                        node = next,
                        via = r,
                        parent = cur,
                        stepRoad = r,
                        stepForward = fwd
                    }
                    __continue16 = true
                until true
                if not __continue16 then
                    break
                end
            end
        end
    end
    return nil
end
function Router.prototype.canTurn(self, n, from, to, forward)
    local outLanes = forward and to.lanesAB or to.lanesBA
    if #outLanes == 0 then
        return false
    end
    if (forward and to.a or to.b) ~= n then
        return false
    end
    if from == nil then
        return true
    end
    local inLanes = from:lanesInto(n)
    for ____, l in ipairs(inLanes) do
        for ____, c in ipairs(l.outgoing) do
            if c.to.road == to then
                return true
            end
        end
    end
    return false
end
function Router.prototype.turnCost(self, n, from, to)
    if from == nil then
        return 0
    end
    for ____, l in ipairs(from:lanesInto(n)) do
        for ____, c in ipairs(l.outgoing) do
            if c.to.road == to then
                return c.turn == "left" and 8 or (c.turn == "right" and 3 or 0)
            end
        end
    end
    return 0
end
return ____exports
 end,
["src.core.vehicle.specs"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__ArrayConcat = ____lualib.__TS__ArrayConcat
local ____exports = {}
local civColors = {
    15263976,
    1710618,
    9080729,
    3820139,
    8003359,
    2841914,
    13218442,
    2763317,
    14209216,
    4927326
}
local brightColors = {
    16722541,
    383464,
    16369678,
    16739345,
    8073207,
    130953,
    14737632,
    1118481
}
____exports.SPECS = {
    compact = {
        id = "compact",
        name = "Makigai-style Compact",
        length = 3.9,
        width = 1.72,
        height = 1.45,
        wheelbase = 2.5,
        frontWeight = 0.6,
        cgHeight = 0.5,
        mass = 1100,
        wheelRadius = 0.29,
        idleRpm = 800,
        redline = 6500,
        torqueCurve = {
            {800, 90},
            {2000, 125},
            {3500, 140},
            {5000, 132},
            {6500, 105}
        },
        engineBrake = 35,
        gears = {
            3.5,
            2,
            1.35,
            1,
            0.8
        },
        reverseRatio = 3.3,
        finalDrive = 4.1,
        shiftTime = 0.35,
        drive = "fwd",
        brakeTorque = 4200,
        brakeBias = 0.68,
        grip = 1,
        tireB = 9,
        tireC = 1.4,
        dragArea = 0.66,
        rollResist = 0.013,
        maxSteer = 0.62,
        fuelCapacity = 40,
        powerKw = 75,
        emergency = false,
        cylinders = 4,
        colors = __TS__ArrayConcat(civColors, {15917388, 7321561}),
        spawnWeight = 22
    },
    sedan = {
        id = "sedan",
        name = "Archer-style Sedan",
        length = 4.8,
        width = 1.85,
        height = 1.45,
        wheelbase = 2.85,
        frontWeight = 0.57,
        cgHeight = 0.52,
        mass = 1550,
        wheelRadius = 0.32,
        idleRpm = 750,
        redline = 6300,
        torqueCurve = {
            {750, 160},
            {2000, 240},
            {4000, 260},
            {5500, 240},
            {6300, 200}
        },
        engineBrake = 45,
        gears = {
            3.6,
            2.2,
            1.5,
            1.15,
            0.92,
            0.75
        },
        reverseRatio = 3.4,
        finalDrive = 3.7,
        shiftTime = 0.3,
        drive = "fwd",
        brakeTorque = 6200,
        brakeBias = 0.66,
        grip = 1,
        tireB = 9,
        tireC = 1.45,
        dragArea = 0.62,
        rollResist = 0.012,
        maxSteer = 0.6,
        fuelCapacity = 60,
        powerKw = 140,
        emergency = false,
        cylinders = 4,
        colors = civColors,
        spawnWeight = 30
    },
    sports = {
        id = "sports",
        name = "Quadra-style Sports Coupe",
        length = 4.5,
        width = 1.95,
        height = 1.22,
        wheelbase = 2.65,
        frontWeight = 0.48,
        cgHeight = 0.42,
        mass = 1380,
        wheelRadius = 0.34,
        idleRpm = 900,
        redline = 8000,
        torqueCurve = {
            {900, 250},
            {3000, 420},
            {5500, 470},
            {7000, 440},
            {8000, 380}
        },
        engineBrake = 60,
        gears = {
            3.1,
            2.1,
            1.6,
            1.28,
            1.05,
            0.86
        },
        reverseRatio = 3,
        finalDrive = 3.6,
        shiftTime = 0.12,
        drive = "rwd",
        brakeTorque = 9000,
        brakeBias = 0.62,
        grip = 1.15,
        tireB = 10,
        tireC = 1.5,
        dragArea = 0.55,
        rollResist = 0.011,
        maxSteer = 0.56,
        fuelCapacity = 65,
        powerKw = 360,
        emergency = false,
        cylinders = 8,
        colors = brightColors,
        spawnWeight = 6
    },
    muscle = {
        id = "muscle",
        name = "Thorton-style Muscle Car",
        length = 4.9,
        width = 1.95,
        height = 1.35,
        wheelbase = 2.9,
        frontWeight = 0.55,
        cgHeight = 0.5,
        mass = 1650,
        wheelRadius = 0.34,
        idleRpm = 700,
        redline = 6200,
        torqueCurve = {
            {700, 380},
            {2500, 560},
            {4000, 600},
            {5500, 540},
            {6200, 470}
        },
        engineBrake = 70,
        gears = {
            2.9,
            1.9,
            1.35,
            1,
            0.8
        },
        reverseRatio = 2.9,
        finalDrive = 3.3,
        shiftTime = 0.25,
        drive = "rwd",
        brakeTorque = 7600,
        brakeBias = 0.64,
        grip = 1,
        tireB = 8.5,
        tireC = 1.45,
        dragArea = 0.72,
        rollResist = 0.013,
        maxSteer = 0.58,
        fuelCapacity = 70,
        powerKw = 320,
        emergency = false,
        cylinders = 8,
        colors = __TS__ArrayConcat(brightColors, civColors),
        spawnWeight = 8
    },
    suv = {
        id = "suv",
        name = "Chevillon-style SUV",
        length = 4.95,
        width = 1.98,
        height = 1.8,
        wheelbase = 2.95,
        frontWeight = 0.55,
        cgHeight = 0.72,
        mass = 2150,
        wheelRadius = 0.37,
        idleRpm = 700,
        redline = 6000,
        torqueCurve = {
            {700, 260},
            {2000, 380},
            {3500, 420},
            {5000, 380},
            {6000, 320}
        },
        engineBrake = 60,
        gears = {
            4,
            2.4,
            1.6,
            1.2,
            1,
            0.8
        },
        reverseRatio = 3.6,
        finalDrive = 3.5,
        shiftTime = 0.3,
        drive = "awd",
        brakeTorque = 9500,
        brakeBias = 0.65,
        grip = 0.95,
        tireB = 8,
        tireC = 1.4,
        dragArea = 1,
        rollResist = 0.015,
        maxSteer = 0.6,
        fuelCapacity = 80,
        powerKw = 210,
        emergency = false,
        cylinders = 6,
        colors = civColors,
        spawnWeight = 16
    },
    van = {
        id = "van",
        name = "Mahir-style Van",
        length = 5.3,
        width = 2,
        height = 2.2,
        wheelbase = 3.3,
        frontWeight = 0.55,
        cgHeight = 0.85,
        mass = 2400,
        wheelRadius = 0.36,
        idleRpm = 750,
        redline = 5000,
        torqueCurve = {
            {750, 260},
            {1800, 380},
            {3000, 390},
            {4200, 340},
            {5000, 280}
        },
        engineBrake = 60,
        gears = {
            4.2,
            2.5,
            1.6,
            1.2,
            1,
            0.82
        },
        reverseRatio = 3.8,
        finalDrive = 3.9,
        shiftTime = 0.35,
        drive = "rwd",
        brakeTorque = 9500,
        brakeBias = 0.65,
        grip = 0.9,
        tireB = 7.5,
        tireC = 1.4,
        dragArea = 1.4,
        rollResist = 0.016,
        maxSteer = 0.62,
        fuelCapacity = 80,
        powerKw = 130,
        emergency = false,
        cylinders = 4,
        colors = {
            16777215,
            13684944,
            2634314,
            9051164,
            14721072
        },
        spawnWeight = 7
    },
    truck = {
        id = "truck",
        name = "Kaukaz-style Box Truck",
        length = 7.8,
        width = 2.45,
        height = 3.4,
        wheelbase = 4.8,
        frontWeight = 0.5,
        cgHeight = 1.2,
        mass = 7500,
        wheelRadius = 0.5,
        idleRpm = 600,
        redline = 2600,
        torqueCurve = {
            {600, 900},
            {1200, 1400},
            {1800, 1450},
            {2300, 1200},
            {2600, 1000}
        },
        engineBrake = 400,
        gears = {
            6.5,
            4,
            2.6,
            1.8,
            1.3,
            1
        },
        reverseRatio = 6,
        finalDrive = 4.1,
        shiftTime = 0.6,
        drive = "rwd",
        brakeTorque = 38000,
        brakeBias = 0.55,
        grip = 0.85,
        tireB = 7,
        tireC = 1.35,
        dragArea = 5.5,
        rollResist = 0.008,
        maxSteer = 0.6,
        fuelCapacity = 300,
        powerKw = 220,
        emergency = false,
        cylinders = 6,
        colors = {15790320, 13647904, 2113648, 3092271},
        spawnWeight = 4
    },
    bus = {
        id = "bus",
        name = "City Bus",
        length = 12,
        width = 2.55,
        height = 3.2,
        wheelbase = 6.2,
        frontWeight = 0.38,
        cgHeight = 1.1,
        mass = 12000,
        wheelRadius = 0.5,
        idleRpm = 600,
        redline = 2400,
        torqueCurve = {
            {600, 1200},
            {1100, 1700},
            {1600, 1750},
            {2100, 1500},
            {2400, 1250}
        },
        engineBrake = 500,
        gears = {
            4.7,
            2.6,
            1.6,
            1.15,
            1,
            0.8
        },
        reverseRatio = 4.5,
        finalDrive = 5.2,
        shiftTime = 0.5,
        drive = "rwd",
        brakeTorque = 60000,
        brakeBias = 0.5,
        grip = 0.85,
        tireB = 7,
        tireC = 1.35,
        dragArea = 7,
        rollResist = 0.008,
        maxSteer = 0.62,
        fuelCapacity = 350,
        powerKw = 230,
        emergency = false,
        cylinders = 6,
        colors = {15911244, 2989275, 14737632},
        spawnWeight = 2
    },
    taxi = {
        id = "taxi",
        name = "Delamain-style Cab",
        length = 4.85,
        width = 1.9,
        height = 1.5,
        wheelbase = 2.85,
        frontWeight = 0.56,
        cgHeight = 0.52,
        mass = 1600,
        wheelRadius = 0.32,
        idleRpm = 750,
        redline = 6300,
        torqueCurve = {
            {750, 200},
            {2000, 300},
            {4000, 320},
            {5500, 290},
            {6300, 250}
        },
        engineBrake = 45,
        gears = {
            3.6,
            2.2,
            1.5,
            1.15,
            0.92,
            0.75
        },
        reverseRatio = 3.4,
        finalDrive = 3.7,
        shiftTime = 0.25,
        drive = "awd",
        brakeTorque = 6600,
        brakeBias = 0.66,
        grip = 1.02,
        tireB = 9,
        tireC = 1.45,
        dragArea = 0.6,
        rollResist = 0.012,
        maxSteer = 0.6,
        fuelCapacity = 60,
        powerKw = 180,
        emergency = false,
        cylinders = 6,
        colors = {15906565, 1118481},
        spawnWeight = 7
    },
    police = {
        id = "police",
        name = "Patrol Interceptor",
        length = 5,
        width = 1.95,
        height = 1.5,
        wheelbase = 2.95,
        frontWeight = 0.54,
        cgHeight = 0.5,
        mass = 1800,
        wheelRadius = 0.34,
        idleRpm = 750,
        redline = 6800,
        torqueCurve = {
            {750, 300},
            {2500, 470},
            {4500, 500},
            {6000, 460},
            {6800, 400}
        },
        engineBrake = 55,
        gears = {
            3.2,
            2.1,
            1.5,
            1.15,
            0.92,
            0.75
        },
        reverseRatio = 3.2,
        finalDrive = 3.5,
        shiftTime = 0.18,
        drive = "awd",
        brakeTorque = 9800,
        brakeBias = 0.63,
        grip = 1.1,
        tireB = 9.5,
        tireC = 1.5,
        dragArea = 0.66,
        rollResist = 0.012,
        maxSteer = 0.6,
        fuelCapacity = 75,
        powerKw = 300,
        emergency = true,
        cylinders = 8,
        colors = {1054752},
        spawnWeight = 0
    },
    ambulance = {
        id = "ambulance",
        name = "Trauma Response Unit",
        length = 6.4,
        width = 2.2,
        height = 2.7,
        wheelbase = 3.9,
        frontWeight = 0.5,
        cgHeight = 1,
        mass = 4200,
        wheelRadius = 0.4,
        idleRpm = 700,
        redline = 5000,
        torqueCurve = {
            {700, 450},
            {1800, 640},
            {3000, 660},
            {4200, 580},
            {5000, 500}
        },
        engineBrake = 120,
        gears = {
            4,
            2.4,
            1.6,
            1.2,
            1,
            0.8
        },
        reverseRatio = 3.8,
        finalDrive = 4.1,
        shiftTime = 0.35,
        drive = "rwd",
        brakeTorque = 20000,
        brakeBias = 0.6,
        grip = 0.9,
        tireB = 7.5,
        tireC = 1.4,
        dragArea = 2.6,
        rollResist = 0.014,
        maxSteer = 0.62,
        fuelCapacity = 120,
        powerKw = 240,
        emergency = true,
        cylinders = 8,
        colors = {16053492},
        spawnWeight = 0
    }
}
____exports.CIVILIAN_CLASSES = {
    "compact",
    "sedan",
    "sports",
    "muscle",
    "suv",
    "van",
    "truck",
    "bus",
    "taxi"
}
--- Approximate max acceleration (m/s^2) at speed v, used by lane-mode AI.
function ____exports.maxAccelAt(spec, v)
    local traction = spec.grip * 9.81 * (spec.drive == "awd" and 0.95 or (spec.drive == "rwd" and 1 - spec.frontWeight + 0.08 or spec.frontWeight - 0.06))
    local power = spec.powerKw * 1000 * 0.85 / (spec.mass * math.max(v, 3))
    local drag = 0.5 * 1.2 * spec.dragArea * v * v / spec.mass + spec.rollResist * 9.81
    return math.max(
        0.3,
        math.min(traction, power) - drag
    )
end
--- Approximate top speed (m/s) from power and drag.
function ____exports.topSpeed(spec)
    local lo = 1
    local hi = 120
    local p = spec.powerKw * 1000 * 0.85
    do
        local i = 0
        while i < 40 do
            local v = (lo + hi) / 2
            local need = (0.5 * 1.2 * spec.dragArea * v * v + spec.rollResist * spec.mass * 9.81) * v
            if need > p then
                hi = v
            else
                lo = v
            end
            i = i + 1
        end
    end
    return lo
end
return ____exports
 end,
["src.core.vehicle.damage"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local ____exports = {}
local ____scalar = require("src.core.math.scalar")
local clamp = ____scalar.clamp
--- Per-zone body damage plus derived mechanical effects.
____exports.DamageModel = __TS__Class()
local DamageModel = ____exports.DamageModel
DamageModel.name = "DamageModel"
function DamageModel.prototype.____constructor(self)
    self.front = 1
    self.rear = 1
    self.left = 1
    self.right = 1
    self.impacts = 0
end
function DamageModel.prototype.body(self)
    return (self.front + self.rear + self.left + self.right) / 4
end
function DamageModel.prototype.totaled(self)
    return self:body() < 0.2 or self.front < 0.05
end
function DamageModel.prototype.hit(self, localX, localY, halfLen, halfWid, severity)
    local nx = localX / halfLen
    local ny = localY / halfWid
    local zone
    if math.abs(nx) >= math.abs(ny) then
        zone = nx >= 0 and "front" or "rear"
    else
        zone = ny >= 0 and "left" or "right"
    end
    local lost = clamp((severity - 1.5) / 14, 0, 1)
    if lost > 0 then
        if zone == "front" then
            self.front = math.max(0, self.front - lost)
        elseif zone == "rear" then
            self.rear = math.max(0, self.rear - lost)
        elseif zone == "left" then
            self.left = math.max(0, self.left - lost)
        else
            self.right = math.max(0, self.right - lost)
        end
        self.impacts = self.impacts + severity
    end
    return {zone = zone, lost = lost}
end
function DamageModel.prototype.repair(self)
    self.front = 1
    self.rear = 1
    self.left = 1
    self.right = 1
end
return ____exports
 end,
["src.core.vehicle.dynamics"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local __TS__MathSign = ____lualib.__TS__MathSign
local ____exports = {}
local ____scalar = require("src.core.math.scalar")
local clamp = ____scalar.clamp
local G = ____scalar.G
local lerp = ____scalar.lerp
local RHO = 1.2
--- Planar rigid-body vehicle model ("bicycle" model with longitudinal load
-- transfer). Each axle is a lumped tyre with a magic-formula lateral curve and
-- a traction-limited longitudinal force, combined through a friction circle.
-- Wheel lock (no ABS) and wheelspin (no TCS) degrade lateral grip, so drifts,
-- burnouts and lock-up understeer emerge naturally.
____exports.VehicleDynamics = __TS__Class()
local VehicleDynamics = ____exports.VehicleDynamics
VehicleDynamics.name = "VehicleDynamics"
function VehicleDynamics.prototype.____constructor(self, spec)
    self.spec = spec
    self.x = 0
    self.y = 0
    self.yaw = 0
    self.u = 0
    self.v = 0
    self.r = 0
    self.steerAngle = 0
    self.ax = 0
    self.ay = 0
    self.omegaF = 0
    self.omegaR = 0
    self.spinSlip = 0
    self.absActive = false
    self.tcsActive = false
    self.escActive = false
    self.frontLocked = false
    self.rearLocked = false
    self.wheelspin = false
    self.skid = 0
    self.slipAngleF = 0
    self.slipAngleR = 0
    self.surfaceGrip = 1
    self.surfaceDrag = 0
    self.tireHealth = 1
    self.mass = spec.mass
    self.iz = spec.mass * (spec.length * spec.length + spec.width * spec.width) / 12 * 0.85
    self.wheelbase = spec.wheelbase
    self.a = spec.wheelbase * (1 - spec.frontWeight)
    self.b = spec.wheelbase * spec.frontWeight
end
function VehicleDynamics.prototype.absSpeed(self)
    return math.sqrt(self.u * self.u + self.v * self.v)
end
function VehicleDynamics.prototype.vx(self)
    return self.u * math.cos(self.yaw) - self.v * math.sin(self.yaw)
end
function VehicleDynamics.prototype.vy(self)
    return self.u * math.sin(self.yaw) + self.v * math.cos(self.yaw)
end
function VehicleDynamics.prototype.setWorldVelocity(self, vx, vy)
    local c = math.cos(self.yaw)
    local s = math.sin(self.yaw)
    self.u = vx * c + vy * s
    self.v = -vx * s + vy * c
end
function VehicleDynamics.prototype.reset(self, x, y, yaw, forwardSpeed)
    self.x = x
    self.y = y
    self.yaw = yaw
    self.u = forwardSpeed
    self.v = 0
    self.r = 0
    self.ax = 0
    self.ay = 0
    self.spinSlip = 0
    self.omegaF = forwardSpeed / self.spec.wheelRadius
    self.omegaR = self.omegaF
end
function VehicleDynamics.prototype.drivenOmega(self)
    local d = self.spec.drive
    if d == "fwd" then
        return self.omegaF
    end
    if d == "rwd" then
        return self.omegaR
    end
    return (self.omegaF + self.omegaR) / 2
end
function VehicleDynamics.prototype.applyImpulse(self, jx, jy, px, py)
    local vx = self:vx() + jx / self.mass
    local vy = self:vy() + jy / self.mass
    local rx = px - self.x
    local ry = py - self.y
    self.r = self.r + (rx * jy - ry * jx) / self.iz
    self:setWorldVelocity(vx, vy)
end
function VehicleDynamics.prototype.lateral(self, alpha, fz, mu)
    local B = self.spec.tireB
    local C = self.spec.tireC
    return -mu * fz * math.sin(C * math.atan(B * alpha))
end
function VehicleDynamics.prototype.step(self, dt, inputs, driveTorque, steerTarget, assists)
    local sub = dt > 1 / 90 and math.ceil(dt / (1 / 120)) or 1
    local h = dt / sub
    do
        local i = 0
        while i < sub do
            self:substep(
                h,
                inputs,
                driveTorque,
                steerTarget,
                assists
            )
            i = i + 1
        end
    end
end
function VehicleDynamics.prototype.substep(self, h, inp, driveTorque, steerTarget, assists)
    local spec = self.spec
    local m = self.mass
    local L = self.wheelbase
    local R = spec.wheelRadius
    local mu = spec.grip * self.surfaceGrip * (0.6 + 0.4 * self.tireHealth)
    local muK = mu * 0.78
    local steerRate = 3.2
    local ds = clamp(steerTarget - self.steerAngle, -steerRate * h, steerRate * h)
    self.steerAngle = self.steerAngle + ds
    local delta = self.steerAngle
    local cd = math.cos(delta)
    local sd = math.sin(delta)
    local wt = m * self.ax * spec.cgHeight / L
    local fzF = clamp(m * G * self.b / L - wt, 0.15 * m * G, 0.85 * m * G)
    local fzR = clamp(m * G * self.a / L + wt, 0.15 * m * G, 0.85 * m * G)
    local u = self.u
    local v = self.v
    local r = self.r
    local vyF = v + self.a * r
    local vyR = v - self.b * r
    local longF = u * cd + vyF * sd
    local latF = -u * sd + vyF * cd
    local longR = u
    local latR = vyR
    local vMin = 1.5
    local alphaF = math.atan2(
        latF,
        math.max(
            math.abs(longF),
            vMin
        )
    )
    local alphaR = math.atan2(
        latR,
        math.max(
            math.abs(longR),
            vMin
        )
    )
    self.slipAngleF = alphaF
    self.slipAngleR = alphaR
    local tqF = 0
    local tqR = 0
    if spec.drive == "fwd" then
        tqF = driveTorque
    elseif spec.drive == "rwd" then
        tqR = driveTorque
    else
        tqF = driveTorque * 0.4
        tqR = driveTorque * 0.6
    end
    local brakeT = inp.brake * spec.brakeTorque
    local bF = brakeT * spec.brakeBias / R
    local bR = brakeT * (1 - spec.brakeBias) / R
    local hb = inp.handbrake * spec.brakeTorque * 0.9
    bR = bR + hb / R
    local rollF = spec.rollResist * fzF * (1 + self.surfaceDrag)
    local rollR = spec.rollResist * fzR * (1 + self.surfaceDrag)
    local res = self:axle(
        tqF / R,
        bF,
        rollF,
        longF,
        latF,
        alphaF,
        fzF,
        mu,
        muK,
        assists,
        false,
        inp.handbrake
    )
    local fxF = res.fx
    local fyF = res.fy
    self.frontLocked = res.locked
    local spinF = res.spin
    local absF = res.abs
    local tcsF = res.tcs
    local resR = self:axle(
        tqR / R,
        bR,
        rollR,
        longR,
        latR,
        alphaR,
        fzR,
        mu,
        muK,
        assists,
        true,
        inp.handbrake
    )
    local fxR = resR.fx
    local fyR = resR.fy
    self.rearLocked = resR.locked
    spinF = spinF or resR.spin
    self.absActive = absF or resR.abs
    self.tcsActive = tcsF or resR.tcs
    self.wheelspin = spinF
    local fxFb = fxF * cd - fyF * sd
    local fyFb = fxF * sd + fyF * cd
    local drag = 0.5 * RHO * spec.dragArea * u * math.abs(u)
    local latDrag = 0.5 * RHO * spec.dragArea * 3 * v * math.abs(v)
    local fx = fxFb + fxR - drag
    local fy = fyFb + fyR - latDrag
    local mz = self.a * fyFb - self.b * fyR
    self.escActive = false
    if assists.esc and math.abs(u) > 6 and inp.handbrake < 0.1 then
        local rDes = u * math.tan(delta) / (L * (1 + 0.0025 * u * u))
        local rMax = mu * G * 0.9 / math.abs(u)
        local target = clamp(rDes, -rMax, rMax)
        local err = r - target
        if math.abs(err) > 0.12 then
            mz = mz - clamp(err * self.iz * 4, -m * G * 1.2, m * G * 1.2)
            fx = fx - __TS__MathSign(u) * math.min(
                math.abs(err),
                1
            ) * m * 1.5
            self.escActive = true
        end
    end
    local du = fx / m + v * r
    local dv = fy / m - u * r
    local dr = mz / self.iz
    local uN = u + du * h
    local vN = v + dv * h
    local rN = r + dr * h
    local driveF = math.abs((tqF + tqR) / R)
    local resist = bF + bR + rollF + rollR
    if u > 0 and uN < 0 or u < 0 and uN > 0 then
        if driveF < resist or __TS__MathSign(driveTorque) == __TS__MathSign(u) then
            uN = 0
        end
    end
    if math.abs(uN) < 0.05 and driveF <= resist + 1 then
        uN = 0
        if math.abs(vN) < 0.3 then
            vN = vN * 0.5
        end
    end
    local spd = math.abs(uN)
    local k = clamp(1 - spd / 3, 0, 1)
    if k > 0 then
        local rKin = uN * math.tan(delta) / L
        local vKin = rKin * self.b
        local blend = math.min(1, k * 12 * h)
        rN = lerp(rN, rKin, blend)
        vN = lerp(vN, vKin, blend)
    end
    du = (uN - u) / h
    dv = (vN - v) / h
    self.u = uN
    self.v = vN
    self.r = rN
    local cy = math.cos(self.yaw)
    local sy = math.sin(self.yaw)
    self.x = self.x + (uN * cy - vN * sy) * h
    self.y = self.y + (uN * sy + vN * cy) * h
    self.yaw = self.yaw + rN * h
    local accLong = du - vN * rN
    local accLat = dv + uN * rN
    local sm = math.min(1, 10 * h)
    self.ax = self.ax + (clamp(accLong, -15, 15) - self.ax) * sm
    self.ay = self.ay + (clamp(accLat, -15, 15) - self.ay) * sm
    if spinF then
        self.spinSlip = math.min(self.spinSlip + 40 * h, 25)
    else
        self.spinSlip = math.max(0, self.spinSlip - 60 * h)
    end
    local sign = driveTorque >= 0 and 1 or -1
    local freeF = longF / R
    local freeR = longR / R
    local driven = spec.drive
    self.omegaF = self.frontLocked and 0 or freeF + (driven ~= "rwd" and sign * self.spinSlip / R or 0)
    self.omegaR = self.rearLocked and 0 or freeR + (driven ~= "fwd" and sign * self.spinSlip / R or 0)
    local peak = 0.12
    local latSkid = math.max(
        math.abs(alphaF),
        math.abs(alphaR)
    ) - peak
    local skid = clamp(latSkid * 5, 0, 1) * clamp(
        self:absSpeed() / 6,
        0,
        1
    )
    if (self.frontLocked or self.rearLocked) and self:absSpeed() > 1 then
        skid = math.max(skid, 0.9)
    end
    if spinF then
        skid = math.max(skid, 0.7)
    end
    self.skid = skid
end
function VehicleDynamics.prototype.axle(self, driveF, brakeF, rollF, vLong, vLat, alpha, fz, mu, muK, assists, rear, handbrake)
    local fMax = mu * fz
    local dir = vLong > 0.2 and 1 or (vLong < -0.2 and -1 or 0)
    local fyPure = self:lateral(alpha, fz, mu)
    if math.abs(vLat) < 0.05 and math.abs(vLong) < 0.2 then
        fyPure = 0
    end
    local fx = driveF
    local locked = false
    local spin = false
    local abs = false
    local tcs = false
    local resist = brakeF + rollF
    if dir ~= 0 then
        fx = fx - dir * resist
    else
        if math.abs(fx) <= resist then
            fx = 0
        else
            fx = fx - __TS__MathSign(fx) * resist
        end
    end
    local brakeDominant = brakeF > math.abs(driveF) and dir ~= 0
    if math.abs(fx) > fMax then
        if brakeDominant then
            local handbrakeLock = rear and handbrake > 0.3
            if assists.abs and not handbrakeLock then
                fx = __TS__MathSign(fx) * fMax * 0.97
                abs = true
            else
                locked = true
            end
        elseif assists.tcs then
            fx = __TS__MathSign(fx) * fMax * 0.95
            tcs = true
        else
            spin = true
        end
    end
    if locked then
        local sp = math.sqrt(vLong * vLong + vLat * vLat)
        if sp > 0.01 then
            return {
                fx = -muK * fz * vLong / sp,
                fy = -muK * fz * vLat / sp,
                locked = locked,
                spin = spin,
                abs = abs,
                tcs = tcs
            }
        end
        return {
            fx = 0,
            fy = 0,
            locked = locked,
            spin = spin,
            abs = abs,
            tcs = tcs
        }
    end
    if spin then
        fx = __TS__MathSign(fx) * muK * fz
        local fyMax = math.sqrt(math.max(0, fMax * fMax - fx * fx)) + fMax * 0.15
        return {
            fx = fx,
            fy = clamp(fyPure * 0.55, -fyMax, fyMax),
            locked = locked,
            spin = spin,
            abs = abs,
            tcs = tcs
        }
    end
    local fyMax = math.sqrt(math.max(0, fMax * fMax - fx * fx))
    local fy = clamp(fyPure, -fyMax, fyMax)
    return {
        fx = fx,
        fy = fy,
        locked = locked,
        spin = spin,
        abs = abs,
        tcs = tcs
    }
end
return ____exports
 end,
["src.core.vehicle.lights"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local ____exports = {}
local ____scalar = require("src.core.math.scalar")
local mod = ____scalar.mod
--- Lamps, signals and other driver-operated equipment: indicators with
-- steering-column auto-cancel, hazards, low/high beam, flash-to-pass, brake
-- and reverse lamps, horn, wipers and emergency lightbar/siren.
____exports.VehicleLights = __TS__Class()
local VehicleLights = ____exports.VehicleLights
VehicleLights.name = "VehicleLights"
function VehicleLights.prototype.____constructor(self, seed)
    self.headlightSwitch = "auto"
    self.highBeam = false
    self.flash = false
    self.indicator = "off"
    self.hazard = false
    self.brake = 0
    self.reverse = false
    self.horn = false
    self.hornTime = 0
    self.hornPresses = 0
    self.wipers = "off"
    self.wiperPos = 0
    self.wiperTimer = 0
    self.wiperDir = 0
    self.siren = "off"
    self.lightbar = false
    self.leftHeadlightBroken = false
    self.rightHeadlightBroken = false
    self.clock = 0
    self.cancelArmed = false
    self.dark = false
    self.indicatorTime = 0
    self.phase = mod(seed * 0.618, 1) * 0.8
end
function VehicleLights.prototype.blinkOn(self)
    return mod(self.clock + self.phase, 0.7) < 0.38
end
function VehicleLights.prototype.leftLampOn(self)
    return (self.hazard or self.indicator == "left") and self:blinkOn()
end
function VehicleLights.prototype.rightLampOn(self)
    return (self.hazard or self.indicator == "right") and self:blinkOn()
end
function VehicleLights.prototype.lowBeamOn(self)
    if self.headlightSwitch == "low" then
        return true
    end
    return self.headlightSwitch == "auto" and self.dark
end
function VehicleLights.prototype.highBeamOn(self)
    return self.flash or self.highBeam and self:lowBeamOn()
end
function VehicleLights.prototype.anyHeadlight(self)
    return self:lowBeamOn() or self:highBeamOn()
end
function VehicleLights.prototype.lightbarPhase(self)
    return mod(self.clock * 3, 1) < 0.5 and 0 or 1
end
function VehicleLights.prototype.setIndicator(self, dir)
    if dir == self.indicator then
        return
    end
    self.indicator = dir
    self.cancelArmed = false
    self.indicatorTime = 0
end
function VehicleLights.prototype.toggleIndicator(self, dir)
    self:setIndicator(self.indicator == dir and "off" or dir)
end
function VehicleLights.prototype.cycleHeadlights(self)
    self.headlightSwitch = self.headlightSwitch == "off" and "auto" or (self.headlightSwitch == "auto" and "low" or "off")
end
function VehicleLights.prototype.cycleWipers(self)
    self.wipers = self.wipers == "off" and "intermittent" or (self.wipers == "intermittent" and "low" or (self.wipers == "low" and "high" or "off"))
end
function VehicleLights.prototype.cycleSiren(self)
    self.siren = self.siren == "off" and "wail" or (self.siren == "wail" and "yelp" or "off")
    self.lightbar = self.siren ~= "off" or self.lightbar
    if self.siren == "off" then
        self.lightbar = false
    end
end
function VehicleLights.prototype.update(self, dt, steer)
    self.clock = self.clock + dt
    if self.horn then
        self.hornTime = self.hornTime + dt
    else
        self.hornTime = 0
    end
    if self.indicator ~= "off" then
        self.indicatorTime = self.indicatorTime + dt
        local toward = self.indicator == "left" and steer or -steer
        if toward > 0.18 then
            self.cancelArmed = true
        elseif self.cancelArmed and toward < 0.04 then
            self.indicator = "off"
            self.cancelArmed = false
            self.indicatorTime = 0
        end
    end
    local sweep = self.wipers == "high" and 0.7 or 1.1
    if self.wipers == "off" and self.wiperDir == 0 then
        self.wiperPos = 0
        return
    end
    if self.wiperDir == 0 then
        self.wiperTimer = self.wiperTimer - dt
        if self.wiperTimer <= 0 and self.wipers ~= "off" then
            self.wiperDir = 1
        end
    else
        self.wiperPos = self.wiperPos + self.wiperDir * 2 / sweep * dt
        if self.wiperPos >= 1 then
            self.wiperPos = 1
            self.wiperDir = -1
        elseif self.wiperPos <= 0 then
            self.wiperPos = 0
            self.wiperDir = 0
            self.wiperTimer = self.wipers == "intermittent" and 3 or 0
        end
    end
end
return ____exports
 end,
["src.core.vehicle.powertrain"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local __TS__MathSign = ____lualib.__TS__MathSign
local ____exports = {}
local ____scalar = require("src.core.math.scalar")
local clamp = ____scalar.clamp
local curve = ____scalar.curve
local RPM_PER_RADS = 60 / (2 * math.pi)
--- Engine, clutch and gearbox. Produces drive torque at the wheels from pedal
-- inputs and the current wheel speed, and keeps engine rpm for HUD/audio.
____exports.Powertrain = __TS__Class()
local Powertrain = ____exports.Powertrain
Powertrain.name = "Powertrain"
function Powertrain.prototype.____constructor(self, spec)
    self.spec = spec
    self.gear = 1
    self.mode = "automatic"
    self.running = true
    self.stalled = false
    self.clutchPedal = 0
    self.health = 1
    self.shiftTimer = 0
    self.pendingGear = 1
    self.limiterActive = false
    self.limiterTimer = 0
    self.engineTorque = 0
    self.load = 0
    self.cranking = 0
    self.fuelUsed = 0
    self.limiter = 0
    self.maxGear = #spec.gears
    self.rpm = spec.idleRpm
    self.fuel = spec.fuelCapacity * 0.75
end
function Powertrain.prototype.ratio(self, g)
    if g == 0 then
        return 0
    end
    if g < 0 then
        return -self.spec.reverseRatio
    end
    return self.spec.gears[g]
end
function Powertrain.prototype.shifting(self)
    return self.shiftTimer > 0
end
function Powertrain.prototype.shiftTo(self, g)
    local target = clamp(
        math.floor(g),
        -1,
        self.maxGear
    )
    if target == self.gear and self.shiftTimer <= 0 then
        return false
    end
    self.pendingGear = target
    self.shiftTimer = self.spec.shiftTime
    return true
end
function Powertrain.prototype.shiftUp(self)
    local base = self.shiftTimer > 0 and self.pendingGear or self.gear
    if base >= self.maxGear then
        return false
    end
    return self:shiftTo(base + 1)
end
function Powertrain.prototype.shiftDown(self)
    local base = self.shiftTimer > 0 and self.pendingGear or self.gear
    if base <= -1 then
        return false
    end
    return self:shiftTo(base - 1)
end
function Powertrain.prototype.gearLabel(self)
    local g = self.shiftTimer > 0 and self.pendingGear or self.gear
    if g < 0 then
        return "R"
    end
    if g == 0 then
        return "N"
    end
    return tostring(g)
end
function Powertrain.prototype.start(self)
    if self.health <= 0.02 or self.fuel <= 0 then
        self.cranking = 1.2
        return
    end
    if not self.running then
        self.cranking = 0.8
    end
end
function Powertrain.prototype.stop(self)
    self.running = false
    self.cranking = 0
end
function Powertrain.prototype.torqueAt(self, rpm, throttle)
    local full = curve(self.spec.torqueCurve, rpm)
    local healthFactor = 0.35 + 0.65 * clamp(self.health, 0, 1)
    local drag = self.spec.engineBrake * clamp((rpm - self.spec.idleRpm * 1.05) / (self.spec.redline - self.spec.idleRpm), 0, 1.2)
    return throttle * full * healthFactor - (1 - throttle) * drag
end
function Powertrain.prototype.update(self, dt, wheelOmega, throttle, speed)
    local spec = self.spec
    if self.cranking > 0 then
        self.cranking = self.cranking - dt
        self.rpm = 280 + 60 * math.sin(self.cranking * 47)
        if self.cranking <= 0 then
            if self.health > 0.02 and self.fuel > 0 then
                self.running = true
                self.stalled = false
                self.rpm = spec.idleRpm * 1.4
            end
        end
        return 0
    end
    if self.fuel <= 0 or self.health <= 0.02 then
        self.running = false
    end
    if self.shiftTimer > 0 then
        self.shiftTimer = self.shiftTimer - dt
        if self.shiftTimer <= 0 then
            self.gear = self.pendingGear
        end
    end
    if self.mode == "automatic" and self.running then
        self:autoShift(throttle, speed)
    end
    if self.limiter > 0 and math.abs(speed) > self.limiter then
        throttle = 0
    end
    local ratio = self:ratio(self.gear) * spec.finalDrive
    local engaged = self.gear ~= 0 and self.shiftTimer <= 0
    local couple = engaged and 1 - self.clutchPedal or 0
    if not self.running then
        self.rpm = math.max(0, self.rpm - 3000 * dt)
        self.engineTorque = 0
        self.load = 0
        return engaged and -__TS__MathSign(wheelOmega) * math.min(
            math.abs(wheelOmega) * 4,
            40
        ) * math.abs(ratio) * couple or 0
    end
    local wheelRpm = math.abs(wheelOmega * ratio) * RPM_PER_RADS
    local freeRev = spec.idleRpm + throttle * (spec.redline - spec.idleRpm)
    local targetRpm
    if couple > 0.05 then
        if self.mode == "automatic" then
            local stall = spec.idleRpm + throttle * math.min(2400, (spec.redline - spec.idleRpm) * 0.45)
            targetRpm = math.max(wheelRpm, stall, spec.idleRpm)
        else
            local slip = 1 - couple
            targetRpm = wheelRpm * couple + freeRev * slip
            if wheelRpm < spec.idleRpm * 0.55 and couple > 0.9 and throttle < 0.6 and math.abs(speed) < 2.5 and self.gear ~= 0 then
                self.running = false
                self.stalled = true
                return 0
            end
            targetRpm = math.max(targetRpm, spec.idleRpm * (couple < 1 and 1 or 0.6))
        end
    else
        targetRpm = freeRev
        couple = 0
    end
    local rate = couple > 0.05 and 25 or 8
    self.rpm = self.rpm + (targetRpm - self.rpm) * math.min(1, rate * dt)
    if self.rpm >= spec.redline then
        self.limiterTimer = 0.08
    end
    local effThrottle = throttle
    if self.limiterTimer > 0 then
        self.limiterTimer = self.limiterTimer - dt
        effThrottle = 0
        self.limiterActive = true
    else
        self.limiterActive = false
    end
    self.rpm = math.min(self.rpm, spec.redline + 150)
    local tq = self:torqueAt(self.rpm, effThrottle)
    if self.mode == "automatic" and couple > 0 and effThrottle < 0.05 and wheelRpm < spec.idleRpm * 1.3 then
        tq = math.max(
            tq,
            curve(spec.torqueCurve, spec.idleRpm) * 0.12
        )
    end
    if self.mode == "automatic" and couple > 0 and wheelRpm < self.rpm * 0.85 then
        tq = tq * (1 + 0.6 * (1 - wheelRpm / math.max(self.rpm, 1)))
    end
    self.engineTorque = tq
    self.load = clamp(effThrottle, 0, 1)
    local powerKw = math.max(0, tq * self.rpm / RPM_PER_RADS / 1000)
    local used = (powerKw * 0.33 + 0.8) * (dt / 3600)
    self.fuel = math.max(0, self.fuel - used)
    self.fuelUsed = self.fuelUsed + used
    return tq * ratio * 0.9 * couple
end
function Powertrain.prototype.autoShift(self, throttle, speed)
    if self.shiftTimer > 0 then
        return
    end
    local spec = self.spec
    if self.gear <= 0 then
        return
    end
    local ratio = self:ratio(self.gear) * spec.finalDrive
    local rpm = math.abs(speed) / spec.wheelRadius * ratio * RPM_PER_RADS
    local upAt = spec.idleRpm + (spec.redline - spec.idleRpm) * (0.35 + 0.6 * throttle)
    local downAt = spec.idleRpm * 1.25 + (spec.redline - spec.idleRpm) * 0.28 * throttle
    if rpm > upAt and self.gear < self.maxGear then
        self:shiftTo(self.gear + 1)
    elseif self.gear > 1 then
        local lowerRatio = self:ratio(self.gear - 1) * spec.finalDrive
        local rpmLower = math.abs(speed) / spec.wheelRadius * lowerRatio * RPM_PER_RADS
        if rpm < downAt or throttle > 0.9 and rpmLower < spec.redline * 0.82 and rpm < spec.redline * 0.55 then
            self:shiftTo(self.gear - 1)
        end
    end
end
return ____exports
 end,
["src.core.vehicle.vehicle"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local __TS__New = ____lualib.__TS__New
local ____exports = {}
local ____scalar = require("src.core.math.scalar")
local clamp = ____scalar.clamp
local ____damage = require("src.core.vehicle.damage")
local DamageModel = ____damage.DamageModel
local ____dynamics = require("src.core.vehicle.dynamics")
local VehicleDynamics = ____dynamics.VehicleDynamics
local ____lights = require("src.core.vehicle.lights")
local VehicleLights = ____lights.VehicleLights
local ____powertrain = require("src.core.vehicle.powertrain")
local Powertrain = ____powertrain.Powertrain
local tmp = {x = 0, y = 0, heading = 0}
____exports.Vehicle = __TS__Class()
local Vehicle = ____exports.Vehicle
Vehicle.name = "Vehicle"
function Vehicle.prototype.____constructor(self, id, spec, color, role)
    self.id = id
    self.spec = spec
    self.color = color
    self.role = role
    self.mode = "lane"
    self.x = 0
    self.y = 0
    self.heading = 0
    self.speed = 0
    self.accel = 0
    self.latAccel = 0
    self.yawRate = 0
    self.wheelAngle = 0
    self.wheelSpin = 0
    self.seg = nil
    self.prevSeg = nil
    self.s = 0
    self.lat = 0
    self.latVel = 0
    self.inputs = {throttle = 0, brake = 0, steer = 0, handbrake = 0}
    self.steerTarget = 0
    self.assists = {abs = true, tcs = true, esc = true}
    self.damage = __TS__New(DamageModel)
    self.alive = true
    self.wrecked = false
    self.stoppedTime = 0
    self.odometer = 0
    self.lastHitTime = -100
    self.lastHitBy = -1
    self.physicsTime = 0
    self.intentConnector = -1
    self.despawnTimer = 0
    self.rpmEstimate = 0
    self.gearEstimate = 1
    self.length = spec.length
    self.width = spec.width
    self.halfLen = spec.length / 2
    self.halfWid = spec.width / 2
    self.dyn = __TS__New(VehicleDynamics, spec)
    self.pt = __TS__New(Powertrain, spec)
    self.lights = __TS__New(VehicleLights, id)
    self.isPlayer = role == "player"
    if spec.emergency then
        self.assists.esc = true
    end
end
function Vehicle.prototype.isEmergency(self)
    return self.role == "police" or self.role == "ambulance"
end
function Vehicle.prototype.sirenOn(self)
    return self.lights.siren ~= "off"
end
function Vehicle.prototype.absSpeed(self)
    return self.mode == "physics" and self.dyn:absSpeed() or math.abs(self.speed)
end
function Vehicle.prototype.vx(self)
    return self.mode == "physics" and self.dyn:vx() or math.cos(self.heading) * self.speed
end
function Vehicle.prototype.vy(self)
    return self.mode == "physics" and self.dyn:vy() or math.sin(self.heading) * self.speed
end
function Vehicle.prototype.placeOnSegment(self, seg, s, speed)
    self.mode = "lane"
    self.seg = seg
    self.s = s
    self.lat = 0
    self.latVel = 0
    self.speed = speed
    self:syncLanePose()
end
function Vehicle.prototype.syncLanePose(self)
    local seg = self.seg
    if seg == nil then
        return
    end
    seg.path:sample(
        clamp(self.s, 0, seg.length),
        tmp
    )
    local x = tmp.x
    local y = tmp.y
    if self.s > seg.length then
        x = x + math.cos(tmp.heading) * (self.s - seg.length)
        y = y + math.sin(tmp.heading) * (self.s - seg.length)
    end
    local nx = -math.sin(tmp.heading)
    local ny = math.cos(tmp.heading)
    self.x = x + nx * self.lat
    self.y = y + ny * self.lat
    local v = math.max(
        math.abs(self.speed),
        1
    )
    local newHeading = tmp.heading + math.atan2(self.latVel, v)
    local d = newHeading - self.heading
    while d > math.pi do
        d = d - math.pi * 2
    end
    while d < -math.pi do
        d = d + math.pi * 2
    end
    self.heading = self.heading + d
end
function Vehicle.prototype.enterPhysics(self)
    if self.mode == "physics" then
        return
    end
    self.dyn:reset(self.x, self.y, self.heading, self.speed)
    self.dyn.v = self.latVel
    self.mode = "physics"
    self.physicsTime = 0
    self.pt.mode = "automatic"
    if self.pt.gear <= 0 then
        self.pt.gear = 1
    end
    self.seg = nil
end
function Vehicle.prototype.enterLane(self, seg, s, lat)
    local sp = self.dyn.u
    self.mode = "lane"
    self.seg = seg
    self.s = s
    self.lat = lat
    self.latVel = 0
    self.speed = math.max(0, sp)
    self:syncLanePose()
end
function Vehicle.prototype.syncFromDynamics(self)
    local d = self.dyn
    self.x = d.x
    self.y = d.y
    self.heading = d.yaw
    self.speed = d.u
    self.accel = d.ax
    self.latAccel = d.ay
    self.yawRate = d.r
    self.wheelAngle = d.steerAngle
end
function Vehicle.prototype.corners(self, out)
    local c = math.cos(self.heading)
    local s = math.sin(self.heading)
    local hl = self.halfLen
    local hw = self.halfWid
    local k = 0
    for ____, sx in ipairs({1, -1}) do
        for ____, sy in ipairs({1, -1}) do
            local lx = sx * hl
            local ly = sx * sy * hw
            out[k + 1] = self.x + lx * c - ly * s
            out[k + 1 + 1] = self.y + lx * s + ly * c
            k = k + 2
        end
    end
    return out
end
return ____exports
 end,
["src.core.ai.personality"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__ArrayMap = ____lualib.__TS__ArrayMap
local ____exports = {}
local ____scalar = require("src.core.math.scalar")
local clamp = ____scalar.clamp
local T = {
    commuter = {label = "Commuter", weight = 40, base = {
        aggression = 0.35,
        patience = 0.55,
        lawfulness = 0.75,
        skill = 0.6,
        attentiveness = 0.8,
        politeness = 0.4,
        speedFactor = 1.03,
        headway = 1.4,
        accelFactor = 1,
        comfortDecel = 2.2,
        minGap = 2.2,
        signalDiscipline = 0.85,
        temper = 1,
        hornHappiness = 0.35,
        criticalGap = 4.5
    }},
    cautious = {label = "Cautious driver", weight = 14, base = {
        aggression = 0.1,
        patience = 0.85,
        lawfulness = 0.95,
        skill = 0.4,
        attentiveness = 0.85,
        politeness = 0.8,
        speedFactor = 0.9,
        headway = 2,
        accelFactor = 0.7,
        comfortDecel = 1.8,
        minGap = 3,
        signalDiscipline = 0.98,
        temper = 0.5,
        hornHappiness = 0.1,
        criticalGap = 6
    }},
    speeder = {label = "Hurried courier", weight = 14, base = {
        aggression = 0.6,
        patience = 0.35,
        lawfulness = 0.45,
        skill = 0.7,
        attentiveness = 0.75,
        politeness = 0.15,
        speedFactor = 1.2,
        headway = 0.9,
        accelFactor = 1.3,
        comfortDecel = 3,
        minGap = 1.6,
        signalDiscipline = 0.5,
        temper = 1.3,
        hornHappiness = 0.6,
        criticalGap = 3.5
    }},
    aggressor = {label = "Hot-head", weight = 8, base = {
        aggression = 0.9,
        patience = 0.12,
        lawfulness = 0.3,
        skill = 0.65,
        attentiveness = 0.7,
        politeness = 0,
        speedFactor = 1.25,
        headway = 0.7,
        accelFactor = 1.4,
        comfortDecel = 3.5,
        minGap = 1.4,
        signalDiscipline = 0.25,
        temper = 2,
        hornHappiness = 0.95,
        criticalGap = 2.8
    }},
    distracted = {label = "Distracted driver", weight = 8, base = {
        aggression = 0.3,
        patience = 0.6,
        lawfulness = 0.7,
        skill = 0.35,
        attentiveness = 0.25,
        politeness = 0.4,
        speedFactor = 0.97,
        headway = 1.5,
        accelFactor = 0.9,
        comfortDecel = 2.2,
        minGap = 2.2,
        signalDiscipline = 0.55,
        temper = 0.9,
        hornHappiness = 0.2,
        criticalGap = 4.5
    }},
    professional = {label = "Professional driver", weight = 8, base = {
        aggression = 0.4,
        patience = 0.65,
        lawfulness = 0.8,
        skill = 0.9,
        attentiveness = 0.9,
        politeness = 0.3,
        speedFactor = 1.05,
        headway = 1.2,
        accelFactor = 1.1,
        comfortDecel = 2.5,
        minGap = 2,
        signalDiscipline = 0.9,
        temper = 0.9,
        hornHappiness = 0.55,
        criticalGap = 3.8
    }},
    joyrider = {label = "Joyrider", weight = 5, base = {
        aggression = 0.75,
        patience = 0.3,
        lawfulness = 0.2,
        skill = 0.55,
        attentiveness = 0.6,
        politeness = 0.05,
        speedFactor = 1.4,
        headway = 0.8,
        accelFactor = 1.5,
        comfortDecel = 3.5,
        minGap = 1.5,
        signalDiscipline = 0.2,
        temper = 1.5,
        hornHappiness = 0.8,
        criticalGap = 2.5
    }},
    zen = {label = "Zen master", weight = 3, base = {
        aggression = 0.02,
        patience = 1,
        lawfulness = 0.9,
        skill = 0.8,
        attentiveness = 0.95,
        politeness = 1,
        speedFactor = 0.98,
        headway = 1.8,
        accelFactor = 0.9,
        comfortDecel = 2,
        minGap = 2.5,
        signalDiscipline = 1,
        temper = 0.2,
        hornHappiness = 0.02,
        criticalGap = 5
    }}
}
____exports.ARCHETYPES = {
    "commuter",
    "cautious",
    "speeder",
    "aggressor",
    "distracted",
    "professional",
    "joyrider",
    "zen"
}
--- Sample a personality; `forced` picks an archetype explicitly.
function ____exports.samplePersonality(rng, forced)
    local entries = __TS__ArrayMap(
        ____exports.ARCHETYPES,
        function(____, a) return {item = a, weight = T[a].weight} end
    )
    local arch = forced or rng:weighted(entries)
    local t = T[arch]
    local function j(v, spread, lo, hi)
        return clamp(
            v + rng:gauss(0, spread),
            lo,
            hi
        )
    end
    local b = t.base
    return {
        archetype = arch,
        label = t.label,
        aggression = j(b.aggression, 0.08, 0, 1),
        patience = j(b.patience, 0.08, 0, 1),
        lawfulness = j(b.lawfulness, 0.07, 0, 1),
        skill = j(b.skill, 0.08, 0.1, 1),
        attentiveness = j(b.attentiveness, 0.06, 0.05, 1),
        politeness = j(b.politeness, 0.08, 0, 1),
        speedFactor = j(b.speedFactor, 0.04, 0.8, 1.55),
        headway = j(b.headway, 0.12, 0.5, 2.6),
        accelFactor = j(b.accelFactor, 0.08, 0.5, 1.6),
        comfortDecel = j(b.comfortDecel, 0.2, 1.4, 4),
        minGap = j(b.minGap, 0.2, 1.2, 3.5),
        signalDiscipline = j(b.signalDiscipline, 0.05, 0, 1),
        temper = j(b.temper, 0.1, 0.1, 2.5),
        hornHappiness = j(b.hornHappiness, 0.06, 0, 1),
        criticalGap = j(b.criticalGap, 0.3, 2, 7)
    }
end
return ____exports
 end,
["src.core.ai.emotion"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local __TS__ArraySplice = ____lualib.__TS__ArraySplice
local ____exports = {}
local ____scalar = require("src.core.math.scalar")
local clamp = ____scalar.clamp
--- Base anger per offense (before temper/stress scaling).
____exports.OFFENSE_ANGER = {
    cut_off = 18,
    tailgating = 4,
    honked_at = 7,
    long_honk = 14,
    high_beams = 7,
    blocked = 2.5,
    collision = 38,
    near_miss = 14,
    rude_gesture = 28,
    brake_check = 22,
    slow_driver = 1.5,
    long_wait = 0.6
}
--- Anger model behind road rage. Offenses add anger (scaled by temper and
-- current stress) and a grudge against the offender; anger decays with
-- patience. Mood thresholds use hysteresis so drivers don't flicker.
____exports.Emotion = __TS__Class()
local Emotion = ____exports.Emotion
Emotion.name = "Emotion"
function Emotion.prototype.____constructor(self, p)
    self.p = p
    self.anger = 0
    self.stress = 0
    self.mood = "calm"
    self.grudges = {}
    self.target = -1
    self.lastOffense = -100
    self.lastOffender = -1
    self.lastKind = nil
    self.forgiven = {}
end
function Emotion.prototype.offend(self, offender, kind, scale, now)
    local forgiveness = 1
    for ____, f in ipairs(self.forgiven) do
        if f.id == offender and now - f.last < 20 then
            forgiveness = 0.4
        end
    end
    local gain = ____exports.OFFENSE_ANGER[kind] * scale * self.p.temper * (1 + self.stress * 0.6) * forgiveness
    self.anger = clamp(self.anger + gain, 0, 100)
    if offender < 0 then
        return gain
    end
    local g = nil
    for ____, e in ipairs(self.grudges) do
        if e.id == offender then
            g = e
        end
    end
    if g == nil then
        g = {id = offender, amount = 0, last = now}
        local ____self_grudges_0 = self.grudges
        ____self_grudges_0[#____self_grudges_0 + 1] = g
    end
    g.amount = math.min(100, g.amount + gain)
    g.last = now
    self.lastOffense = now
    self.lastOffender = offender
    self.lastKind = kind
    self:pickTarget()
    return gain
end
function Emotion.prototype.apologize(self, offender, now)
    local g = nil
    for ____, e in ipairs(self.grudges) do
        if e.id == offender then
            g = e
        end
    end
    if g == nil then
        return 0
    end
    local recent = now - g.last < 10
    local base = recent and 26 or 10
    local calmed = base * (1.2 - self.p.aggression * 0.7)
    local before = self.anger
    self.anger = clamp(self.anger - calmed, 0, 100)
    g.amount = math.max(0, g.amount - calmed * 1.5)
    local ____self_forgiven_1 = self.forgiven
    ____self_forgiven_1[#____self_forgiven_1 + 1] = {id = offender, amount = 0, last = now}
    if #self.forgiven > 6 then
        table.remove(self.forgiven, 1)
    end
    self:pickTarget()
    return before - self.anger
end
function Emotion.prototype.grudgeAgainst(self, id)
    for ____, e in ipairs(self.grudges) do
        if e.id == id then
            return e.amount
        end
    end
    return 0
end
function Emotion.prototype.pickTarget(self)
    local best = 15
    self.target = -1
    for ____, g in ipairs(self.grudges) do
        if g.amount > best then
            best = g.amount
            self.target = g.id
        end
    end
end
function Emotion.prototype.update(self, dt, now, congestion)
    self.stress = self.stress + (clamp(congestion, 0, 1) - self.stress) * math.min(1, dt * 0.05)
    local sinceOffense = now - self.lastOffense
    local decay = (0.6 + self.p.patience * 3.2) * (sinceOffense > 6 and 1.6 or 0.6) * (1 - self.stress * 0.4)
    self.anger = math.max(0, self.anger - decay * dt)
    do
        local i = #self.grudges - 1
        while i >= 0 do
            local g = self.grudges[i + 1]
            g.amount = g.amount - decay * 0.8 * dt
            if g.amount <= 0 then
                __TS__ArraySplice(self.grudges, i, 1)
            end
            i = i - 1
        end
    end
    if self.target >= 0 then
        self:pickTarget()
    end
    local a = self.anger
    local m = self.mood
    if m == "calm" then
        if a > 22 then
            self.mood = "irritated"
        end
    elseif m == "irritated" then
        if a > 48 then
            self.mood = "angry"
        elseif a < 14 then
            self.mood = "calm"
        end
    elseif m == "angry" then
        if a > 74 then
            self.mood = "furious"
        elseif a < 38 then
            self.mood = "irritated"
        end
    elseif a < 60 then
        self.mood = "angry"
    end
end
function Emotion.prototype.canRage(self)
    return self.mood == "furious" and self.p.aggression > 0.55
end
return ____exports
 end,
["src.core.signals.signalController"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local __TS__ArrayForEach = ____lualib.__TS__ArrayForEach
local __TS__ArrayMap = ____lualib.__TS__ArrayMap
local __TS__ArraySome = ____lualib.__TS__ArraySome
local Map = ____lualib.Map
local __TS__New = ____lualib.__TS__New
local __TS__ArrayIndexOf = ____lualib.__TS__ArrayIndexOf
local ____exports = {}
local ____scalar = require("src.core.math.scalar")
local angleDiff = ____scalar.angleDiff
local clamp = ____scalar.clamp
local DEG = ____scalar.DEG
local mod = ____scalar.mod
--- Traffic signal controller for one intersection. Builds a phase plan from the
-- node geometry (opposing approaches share a phase, dedicated left lanes get a
-- leading protected arrow) and runs it fixed-time, actuated, flashing or dark.
____exports.SignalController = __TS__Class()
local SignalController = ____exports.SignalController
SignalController.name = "SignalController"
function SignalController.prototype.____constructor(self, id, node, opts)
    self.id = id
    self.node = node
    self.phases = {}
    self.phaseIndex = 0
    self.state = "green"
    self.t = 0
    self.gapTimer = 0
    self.preemptApproach = -1
    self.preemptTtl = 0
    self.flashClock = 0
    self.mainApproaches = {}
    self.pedWalkTime = 7
    self.onChange = nil
    self.arrowApproaches = {}
    self.mode = opts.mode or "actuated"
    local ____opts_allowRightOnRed_0 = opts.allowRightOnRed
    if ____opts_allowRightOnRed_0 == nil then
        ____opts_allowRightOnRed_0 = true
    end
    self.allowRightOnRed = ____opts_allowRightOnRed_0
    self:buildPhases(opts.cycleScale or 1)
    local offset = opts.offset or 0
    if offset > 0 then
        local saved = self.mode
        self.mode = "fixed"
        local left = offset
        while left > 0 do
            local dt = math.min(0.5, left)
            self:update(dt, nil)
            left = left - dt
        end
        self.mode = saved
    end
end
function SignalController.prototype.buildPhases(self, cycleScale)
    local aps = self.node.approaches
    local used = {}
    do
        local i = 0
        while i < #aps do
            used[#used + 1] = false
            i = i + 1
        end
    end
    local groups = {}
    do
        local i = 0
        while i < #aps do
            do
                local __continue7
                repeat
                    if used[i + 1] then
                        __continue7 = true
                        break
                    end
                    local g = {aps[i + 1]}
                    used[i + 1] = true
                    do
                        local j = i + 1
                        while j < #aps do
                            do
                                local __continue9
                                repeat
                                    if used[j + 1] then
                                        __continue9 = true
                                        break
                                    end
                                    if math.abs(math.abs(angleDiff(aps[i + 1].heading, aps[j + 1].heading)) - math.pi) < 35 * DEG then
                                        g[#g + 1] = aps[j + 1]
                                        used[j + 1] = true
                                        break
                                    end
                                    __continue9 = true
                                until true
                                if not __continue9 then
                                    break
                                end
                            end
                            j = j + 1
                        end
                    end
                    groups[#groups + 1] = g
                    __continue7 = true
                until true
                if not __continue7 then
                    break
                end
            end
            i = i + 1
        end
    end
    local bestW = -1
    for ____, g in ipairs(groups) do
        local w = 0
        for ____, a in ipairs(g) do
            w = w + #a.lanes * (a.road.speedLimit + 1)
        end
        if w > bestW then
            bestW = w
            self.mainApproaches = g
        end
    end
    local gi = 0
    for ____, g in ipairs(groups) do
        __TS__ArrayForEach(
            g,
            function(____, a)
                local ____gi_1 = gi
                a.group = ____gi_1
                return ____gi_1
            end
        )
        local isMain = g == self.mainApproaches
        local speed = math.max(unpack(__TS__ArrayMap(
            g,
            function(____, a) return a.road.speedLimit end
        )))
        local yellow = clamp(1 + speed / (2 * 3.05), 3, 5.5)
        local crossWidth = self.node.radius * 2
        local allRed = clamp(
            (crossWidth + 5) / math.max(speed, 8),
            1,
            2.5
        )
        local hasDedicatedLeft = __TS__ArraySome(
            g,
            function(____, a) return __TS__ArraySome(
                a.lanes,
                function(____, l) return #l.turns == 1 and l.turns[1] == "left" end
            ) end
        )
        if hasDedicatedLeft and #g > 1 then
            local serves = __TS__New(Map)
            for ____, a in ipairs(g) do
                for ____, l in ipairs(a.lanes) do
                    for ____, c in ipairs(l.outgoing) do
                        if c.turn == "left" then
                            serves:set(c.id, "green")
                        end
                    end
                end
            end
            local ____self_phases_2 = self.phases
            ____self_phases_2[#____self_phases_2 + 1] = {
                name = ("G" .. tostring(gi)) .. "-left",
                serves = serves,
                approaches = g,
                isLeftArrow = true,
                minGreen = 4,
                maxGreen = 12 * cycleScale,
                fixedGreen = 8 * cycleScale,
                yellow = 3,
                allRed = 1,
                recall = false
            }
        end
        local serves = __TS__New(Map)
        for ____, a in ipairs(g) do
            local opposed = #g > 1
            for ____, l in ipairs(a.lanes) do
                for ____, c in ipairs(l.outgoing) do
                    if c.turn == "left" or c.turn == "uturn" then
                        serves:set(c.id, opposed and "permissive" or "green")
                    else
                        serves:set(c.id, "green")
                    end
                end
            end
        end
        local ____self_phases_3 = self.phases
        ____self_phases_3[#____self_phases_3 + 1] = {
            name = ("G" .. tostring(gi)) .. "-through",
            serves = serves,
            approaches = g,
            isLeftArrow = false,
            minGreen = isMain and 10 or 6,
            maxGreen = (isMain and 40 or 24) * cycleScale,
            fixedGreen = (isMain and 26 or 16) * cycleScale,
            yellow = yellow,
            allRed = allRed,
            recall = isMain
        }
        gi = gi + 1
    end
    for ____, p in ipairs(self.phases) do
        if p.isLeftArrow then
            for ____, a in ipairs(p.approaches) do
                if __TS__ArrayIndexOf(self.arrowApproaches, a.id) < 0 then
                    local ____self_arrowApproaches_4 = self.arrowApproaches
                    ____self_arrowApproaches_4[#____self_arrowApproaches_4 + 1] = a.id
                end
            end
        end
    end
    do
        local i = 0
        while i < #self.phases do
            if self.phases[i + 1].recall then
                self.phaseIndex = i
                break
            end
            i = i + 1
        end
    end
end
function SignalController.prototype.currentPhase(self)
    return self.phases[self.phaseIndex + 1]
end
function SignalController.prototype.phaseState(self)
    return self.state
end
function SignalController.prototype.stateTime(self)
    return self.t
end
function SignalController.prototype.cycleLength(self)
    local c = 0
    for ____, p in ipairs(self.phases) do
        c = c + (p.fixedGreen + p.yellow + p.allRed)
    end
    return c
end
function SignalController.prototype.setMode(self, mode)
    if mode == self.mode then
        return
    end
    self.mode = mode
    if mode == "fixed" or mode == "actuated" then
        self.state = "allred"
        self.t = 0
    end
end
function SignalController.prototype.requestPreempt(self, approach)
    self.preemptApproach = approach.id
    self.preemptTtl = 1
end
function SignalController.prototype.preempting(self)
    return self.preemptApproach >= 0
end
function SignalController.prototype.phaseServesApproach(self, p, apId)
    if p.isLeftArrow then
        return false
    end
    for ____, a in ipairs(p.approaches) do
        if a.id == apId then
            return true
        end
    end
    return false
end
function SignalController.prototype.phaseDemand(self, p, demand)
    if demand == nil then
        return 1
    end
    local d = 0
    for ____, a in ipairs(p.approaches) do
        for ____, l in ipairs(a.lanes) do
            for ____, c in ipairs(l.outgoing) do
                if p.serves:has(c.id) then
                    d = d + demand(c)
                end
            end
        end
    end
    return d
end
function SignalController.prototype.emit(self)
    if self.onChange ~= nil then
        self.onChange({
            nodeId = self.node.id,
            phase = self:currentPhase().name,
            state = self.state
        })
    end
end
function SignalController.prototype.pickNextPhase(self, demand)
    local n = #self.phases
    if self.preemptApproach >= 0 then
        do
            local k = 0
            while k < n do
                if self:phaseServesApproach(self.phases[k + 1], self.preemptApproach) then
                    return k
                end
                k = k + 1
            end
        end
    end
    if self.mode == "fixed" then
        return mod(self.phaseIndex + 1, n)
    end
    do
        local step = 1
        while step <= n do
            local k = mod(self.phaseIndex + step, n)
            local p = self.phases[k + 1]
            if self:phaseDemand(p, demand) > 0 or p.recall and step == n then
                return k
            end
            step = step + 1
        end
    end
    do
        local k = 0
        while k < n do
            if self.phases[k + 1].recall then
                return k
            end
            k = k + 1
        end
    end
    return mod(self.phaseIndex + 1, n)
end
function SignalController.prototype.greenShouldEnd(self, demand, dt)
    local p = self:currentPhase()
    if self.preemptApproach >= 0 then
        return not self:phaseServesApproach(p, self.preemptApproach) and self.t >= math.min(p.minGreen, 4)
    end
    if self.mode == "fixed" then
        return self.t >= p.fixedGreen
    end
    if self.t < p.minGreen then
        return false
    end
    if self.t >= p.maxGreen then
        return self:othersWaiting(demand)
    end
    local mine = self:phaseDemand(p, demand)
    if mine > 0 then
        self.gapTimer = 0
    else
        self.gapTimer = self.gapTimer + dt
    end
    local passage = p.isLeftArrow and 1.5 or 3
    return self.gapTimer >= passage and self:othersWaiting(demand)
end
function SignalController.prototype.othersWaiting(self, demand)
    do
        local k = 0
        while k < #self.phases do
            if k ~= self.phaseIndex and self:phaseDemand(self.phases[k + 1], demand) > 0 then
                return true
            end
            k = k + 1
        end
    end
    return false
end
function SignalController.prototype.update(self, dt, demand)
    self.flashClock = self.flashClock + dt
    if self.preemptTtl > 0 then
        self.preemptTtl = self.preemptTtl - dt
        if self.preemptTtl <= 0 then
            self.preemptApproach = -1
        end
    end
    if self.mode == "flashing" or self.mode == "off" then
        return
    end
    self.t = self.t + dt
    local p = self:currentPhase()
    if self.state == "green" then
        if self:greenShouldEnd(demand, dt) then
            self.state = "yellow"
            self.t = 0
            self:emit()
        end
    elseif self.state == "yellow" then
        if self.t >= p.yellow then
            self.state = "allred"
            self.t = 0
            self:emit()
        end
    elseif self.t >= p.allRed then
        self.phaseIndex = self:pickNextPhase(demand)
        self.state = "green"
        self.t = 0
        self.gapTimer = 0
        self:emit()
    end
end
function SignalController.prototype.flashOn(self)
    return mod(self.flashClock, 1) < 0.5
end
function SignalController.prototype.isMain(self, a)
    return __TS__ArrayIndexOf(self.mainApproaches, a) >= 0
end
function SignalController.prototype.indication(self, c)
    if self.mode == "off" then
        return "dark"
    end
    if self.mode == "flashing" then
        return self:isMain(c.approach) and "flash_yellow" or "flash_red"
    end
    local p = self:currentPhase()
    local right = p.serves:get(c.id)
    if right == nil or self.state == "allred" then
        return "red"
    end
    if self.state == "yellow" then
        return "yellow"
    end
    return right
end
function SignalController.prototype.timeInState(self)
    return self.t
end
function SignalController.prototype.head(self, a)
    local out = {
        ball = "red",
        arrow = nil,
        ballFlashing = false,
        ped = "dont",
        pedCountdown = 0
    }
    local hasArrow = __TS__ArrayIndexOf(self.arrowApproaches, a.id) >= 0
    if hasArrow then
        out.arrow = "off"
    end
    if self.mode == "off" then
        out.ball = "off"
        out.ped = "off"
        if hasArrow then
            out.arrow = "off"
        end
        return out
    end
    if self.mode == "flashing" then
        out.ballFlashing = true
        out.ball = self:flashOn() and (self:isMain(a) and "yellow" or "red") or "off"
        out.ped = "off"
        return out
    end
    local p = self:currentPhase()
    local inPhase = __TS__ArrayIndexOf(p.approaches, a) >= 0
    local lamp = self.state == "green" and "green" or (self.state == "yellow" and "yellow" or "red")
    if inPhase and self.state ~= "allred" then
        if p.isLeftArrow then
            out.arrow = lamp
            out.ball = "red"
        else
            out.ball = lamp
            if hasArrow then
                out.arrow = "off"
            end
        end
    elseif hasArrow then
        out.arrow = "red"
    end
    if inPhase and not p.isLeftArrow and self.state == "green" then
        local clearance = self:pedClearance()
        local greenLeft = self.mode == "fixed" and p.fixedGreen - self.t or math.max(p.minGreen, clearance + self.pedWalkTime) - self.t
        if self.t < self.pedWalkTime then
            out.ped = "walk"
        elseif greenLeft > 0 then
            out.ped = "flash"
            out.pedCountdown = math.ceil(math.max(0, greenLeft))
        end
    end
    return out
end
function SignalController.prototype.pedClearance(self)
    return self.node.radius * 2 / 1.2
end
function SignalController.prototype.isGreenFor(self, a)
    return self.state == "green" and __TS__ArrayIndexOf(
        self:currentPhase().approaches,
        a
    ) >= 0 and not self:currentPhase().isLeftArrow
end
return ____exports
 end,
["src.core.util.events"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local Map = ____lualib.Map
local __TS__New = ____lualib.__TS__New
local __TS__ArrayIndexOf = ____lualib.__TS__ArrayIndexOf
local __TS__ArraySplice = ____lualib.__TS__ArraySplice
local ____exports = {}
____exports.EventBus = __TS__Class()
local EventBus = ____exports.EventBus
EventBus.name = "EventBus"
function EventBus.prototype.____constructor(self)
    self.listeners = __TS__New(Map)
    self.anyListeners = {}
    self.queue = {}
    self.queueLimit = 512
end
function EventBus.prototype.on(self, ____type, fn)
    local list = self.listeners:get(____type)
    if list == nil then
        list = {}
        self.listeners:set(____type, list)
    end
    list[#list + 1] = fn
    return function()
        local l = self.listeners:get(____type)
        if l ~= nil then
            local i = __TS__ArrayIndexOf(l, fn)
            if i >= 0 then
                __TS__ArraySplice(l, i, 1)
            end
        end
    end
end
function EventBus.prototype.onAny(self, fn)
    local ____self_anyListeners_0 = self.anyListeners
    ____self_anyListeners_0[#____self_anyListeners_0 + 1] = fn
end
function EventBus.prototype.emit(self, event)
    local list = self.listeners:get(event.type)
    if list ~= nil then
        do
            local i = 0
            while i < #list do
                list[i + 1](event)
                i = i + 1
            end
        end
    end
    do
        local i = 0
        while i < #self.anyListeners do
            self.anyListeners[i + 1](event)
            i = i + 1
        end
    end
    if #self.queue < self.queueLimit then
        local ____self_queue_1 = self.queue
        ____self_queue_1[#____self_queue_1 + 1] = event
    end
end
function EventBus.prototype.drain(self)
    local q = self.queue
    self.queue = {}
    return q
end
return ____exports
 end,
["src.core.world.environment"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local __TS__New = ____lualib.__TS__New
local ____exports = {}
local ____scalar = require("src.core.math.scalar")
local clamp = ____scalar.clamp
local mod = ____scalar.mod
local smoothstep = ____scalar.smoothstep
local ____rng = require("src.core.math.rng")
local Rng = ____rng.Rng
local function bell(h, c, w)
    return math.exp(-((h - c) * (h - c)) / (2 * w * w))
end
--- Time of day and weather, with their effects on driving.
____exports.Environment = __TS__Class()
local Environment = ____exports.Environment
Environment.name = "Environment"
function Environment.prototype.____constructor(self, seed, startHour)
    self.timeScale = 1 / 60
    self.weather = "clear"
    self.rain = 0
    self.fog = 0
    self.wetness = 0
    self.weatherTimer = 600
    self.autoWeather = true
    self.daylight = 1
    self.dark = false
    self.grip = 1
    self.visibility = 1200
    self.trafficDensity = 1
    self.nightFlash = false
    self.hour = startHour
    self.rng = __TS__New(Rng, seed)
    self:refresh()
end
function Environment.prototype.refresh(self)
    local h = self.hour
    self.daylight = smoothstep(5.5, 7.5, h) * (1 - smoothstep(18.5, 20.5, h))
    self.dark = self.daylight < 0.45 or self.fog > 0.6
    self.grip = 1 - 0.32 * self.wetness
    local night = 1 - self.daylight
    self.visibility = clamp(1200 * (1 - self.fog * 0.9) * (1 - self.rain * 0.5) * (1 - night * 0.4), 40, 1200)
    self.trafficDensity = clamp(
        0.25 + 0.75 * bell(h, 8.2, 1.4) + 0.85 * bell(h, 17.6, 1.7) + 0.45 * bell(h, 13, 2.5) + 0.25 * bell(h, 21.5, 1.5),
        0.2,
        1.1
    )
    self.nightFlash = h >= 1 and h < 5
end
function Environment.prototype.setWeather(self, w)
    self.weather = w
    self.weatherTimer = 900
end
function Environment.prototype.update(self, dt)
    self.hour = mod(self.hour + dt * self.timeScale, 24)
    if self.autoWeather then
        self.weatherTimer = self.weatherTimer - dt
        if self.weatherTimer <= 0 then
            self.weather = self.rng:weighted({
                {item = "clear", weight = 45},
                {item = "cloudy", weight = 25},
                {item = "rain", weight = 18},
                {item = "storm", weight = 6},
                {item = "fog", weight = 6}
            })
            self.weatherTimer = self.rng:range(400, 1200)
        end
    end
    local targetRain = self.weather == "rain" and 0.6 or (self.weather == "storm" and 1 or 0)
    local targetFog = self.weather == "fog" and 0.8 or (self.weather == "storm" and 0.3 or 0)
    self.rain = self.rain + (targetRain - self.rain) * math.min(1, dt * 0.1)
    self.fog = self.fog + (targetFog - self.fog) * math.min(1, dt * 0.05)
    local wetTarget = self.rain > 0.05 and math.min(1, self.rain * 1.5) or 0
    self.wetness = self.wetness + (wetTarget - self.wetness) * math.min(1, dt * (wetTarget > self.wetness and 0.08 or 0.01))
    self:refresh()
end
return ____exports
 end,
["src.core.world.events"] = function(...) 
--[[ Generated with https://github.com/TypeScriptToLua/TypeScriptToLua ]]
local ____exports = {}
return ____exports
 end,
["src.core.world.context"] = function(...) 
--[[ Generated with https://github.com/TypeScriptToLua/TypeScriptToLua ]]
local ____exports = {}
return ____exports
 end,
["src.core.ai.control"] = function(...) 
--[[ Generated with https://github.com/TypeScriptToLua/TypeScriptToLua ]]
local ____exports = {}
local ____scalar = require("src.core.math.scalar")
local clamp = ____scalar.clamp
local wrapAngle = ____scalar.wrapAngle
--- Low-level controller for physics-mode AI: pure-pursuit steering toward a
-- target point plus a proportional speed controller. Writes vehicle.inputs.
function ____exports.driveToward(v, tx, ty, targetSpeed, allowReverse)
    local d = v.dyn
    local dx = tx - d.x
    local dy = ty - d.y
    local c = math.cos(d.yaw)
    local s = math.sin(d.yaw)
    local lx = dx * c + dy * s
    local ly = -dx * s + dy * c
    local dist = math.sqrt(dx * dx + dy * dy)
    local alpha = math.atan2(ly, lx)
    local reverse = allowReverse and lx < -1 and dist < 12 and math.abs(alpha) > 2.2
    local L = v.spec.wheelbase
    local ld = math.max(dist, 4)
    local steer = math.atan(2 * L * math.sin(alpha) / ld)
    if reverse then
        steer = -math.atan(2 * L * math.sin(wrapAngle(alpha + math.pi)) / ld)
    end
    v.steerTarget = clamp(steer, -v.spec.maxSteer, v.spec.maxSteer)
    local sharp = math.abs(reverse and wrapAngle(alpha + math.pi) or alpha)
    local want = targetSpeed * clamp(1.2 - sharp / 1.6, 0.25, 1)
    if reverse then
        want = -math.min(4, targetSpeed)
    end
    local err = want - d.u
    if reverse then
        if v.pt.gear ~= -1 then
            v.pt:shiftTo(-1)
        end
        v.inputs.throttle = clamp(-err * 0.35, 0, 1)
        v.inputs.brake = clamp(err * 0.5, 0, 1)
    else
        if v.pt.gear <= 0 then
            v.pt:shiftTo(1)
        end
        v.inputs.throttle = clamp(err * 0.35, 0, 1)
        v.inputs.brake = clamp(-err * 0.45, 0, 1)
    end
    v.inputs.handbrake = 0
end
function ____exports.brakeToStop(v, hard)
    v.inputs.throttle = 0
    v.inputs.brake = hard and 1 or 0.6
    v.inputs.handbrake = math.abs(v.dyn.u) < 0.3 and 1 or 0
    v.steerTarget = 0
end
return ____exports
 end,
["src.core.ai.idm"] = function(...) 
--[[ Generated with https://github.com/TypeScriptToLua/TypeScriptToLua ]]
local ____exports = {}
--- IDM free-road term.
function ____exports.idmFree(v, p)
    if p.v0 <= 0.1 then
        return -p.b * 2
    end
    local r = v / p.v0
    if r <= 1 then
        return p.a * (1 - r * r * r * r)
    end
    return -p.b * (1 - 1 / (r * r))
end
--- Full IDM acceleration toward a leader.
-- 
-- @param gap bumper-to-bumper distance (m)
-- @param dv approach rate v - vLeader (m/s)
function ____exports.idmAccel(v, gap, dv, p)
    local free = ____exports.idmFree(v, p)
    local g = math.max(gap, 0.05)
    local sStar = p.s0 + math.max(
        0,
        v * p.T + v * dv / (2 * math.sqrt(p.a * p.b))
    )
    local inter = p.a * (sStar / g) * (sStar / g)
    return free - inter
end
--- Deceleration needed to stop within `dist` from speed v.
function ____exports.decelToStop(v, dist)
    if dist <= 0.05 then
        return v > 0.1 and 99 or 0
    end
    return v * v / (2 * dist)
end
return ____exports
 end,
["src.core.ai.intersection"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local ____exports = {}
local ____scalar = require("src.core.math.scalar")
local angleDiff = ____scalar.angleDiff
local clamp = ____scalar.clamp
local DEG = ____scalar.DEG
local lerp = ____scalar.lerp
--- Per-approach latched decisions (reset whenever the driver enters a new lane).
____exports.JunctionState = __TS__Class()
local JunctionState = ____exports.JunctionState
JunctionState.name = "JunctionState"
function JunctionState.prototype.____constructor(self)
    self.laneId = -1
    self.yellow = nil
    self.redRun = false
    self.hasStopped = false
    self.queued = false
    self.boxIgnore = false
    self.waitTime = 0
    self.reason = ""
    self.lastIndication = nil
end
function JunctionState.prototype.reset(self, laneId)
    self.laneId = laneId
    self.yellow = nil
    self.redRun = false
    self.hasStopped = false
    self.queued = false
    self.boxIgnore = false
    self.waitTime = 0
    self.reason = ""
    self.lastIndication = nil
end
local MOVE_RANK = {straight = 3, right = 2, left = 1, uturn = 0}
local function indRank(i)
    repeat
        local ____switch5 = i
        local ____cond5 = ____switch5 == "green"
        if ____cond5 then
            return 4
        end
        ____cond5 = ____cond5 or ____switch5 == "yellow"
        if ____cond5 then
            return 3
        end
        ____cond5 = ____cond5 or (____switch5 == "permissive" or ____switch5 == "flash_yellow")
        if ____cond5 then
            return 2
        end
        ____cond5 = ____cond5 or (____switch5 == "flash_red" or ____switch5 == "dark")
        if ____cond5 then
            return 1
        end
        do
            return 0
        end
    until true
end
function ____exports.indicationFor(ctx, c)
    local n = c.node
    if n.control ~= "signal" or n.signalId < 0 then
        return nil
    end
    return ctx.signals[n.signalId + 1]:indication(c)
end
--- Does movement `o` have right of way over movement `c` right now?
function ____exports.hasPriority(ctx, o, c)
    local io = ____exports.indicationFor(ctx, o)
    local ic = ____exports.indicationFor(ctx, c)
    if io ~= nil and ic ~= nil then
        local ro = indRank(io)
        local rc = indRank(ic)
        if ro ~= rc then
            return ro > rc
        end
    else
        local mo = o.approach.major
        local mc = c.approach.major
        if mo ~= mc then
            return mo
        end
    end
    local a = MOVE_RANK[o.turn]
    local b = MOVE_RANK[c.turn]
    if a ~= b then
        return a > b
    end
    local d = angleDiff(c.approach.heading, o.approach.heading)
    return d > 45 * DEG and d < 135 * DEG
end
--- Resolve the entry mode for connector c from signals and signs.
function ____exports.entryMode(ctx, d, c, distToLine)
    local n = c.node
    local ap = c.approach
    local v = d.v
    local ind = ____exports.indicationFor(ctx, c)
    if ind ~= nil then
        d.js.lastIndication = ind
        repeat
            local ____switch16 = ind
            local ____cond16 = ____switch16 == "green"
            if ____cond16 then
                d.js.yellow = nil
                return "go"
            end
            ____cond16 = ____cond16 or ____switch16 == "permissive"
            if ____cond16 then
                return "yield"
            end
            ____cond16 = ____cond16 or ____switch16 == "flash_yellow"
            if ____cond16 then
                return "yield"
            end
            ____cond16 = ____cond16 or (____switch16 == "flash_red" or ____switch16 == "dark")
            if ____cond16 then
                return "allway"
            end
            ____cond16 = ____cond16 or ____switch16 == "yellow"
            if ____cond16 then
                do
                    if d.emergency then
                        return "yield"
                    end
                    if d.js.yellow == nil then
                        local sp = math.abs(v.speed)
                        local need = sp * sp / (2 * math.max(distToLine, 0.1))
                        local threshold = lerp(4.6, 2, d.p.aggression) * lerp(0.85, 1.1, d.p.lawfulness)
                        local tooClose = distToLine < sp * 0.8
                        d.js.yellow = need <= threshold and not tooClose and "stop" or "go"
                        if d.mood == "furious" or d.raging then
                            d.js.yellow = need < 1.2 and "stop" or "go"
                        end
                    end
                    return d.js.yellow == "go" and "go" or "stop"
                end
            end
            ____cond16 = ____cond16 or ____switch16 == "red"
            if ____cond16 then
                do
                    if d.emergency then
                        return "stop_then_yield"
                    end
                    if d.js.yellow == "go" and distToLine < 3 then
                        return "go"
                    end
                    if not d.js.redRun and d.js.lastIndication == "red" and distToLine > 25 and math.abs(v.speed) > 8 then
                        local chance = (1 - d.p.lawfulness) * 0.08 + (d.raging and 0.5 or 0) + (d.mood == "furious" and 0.15 or 0)
                        if d:roll() < chance then
                            d.js.redRun = true
                        end
                    end
                    if d.js.redRun then
                        return "yield"
                    end
                    local sig = ctx.signals[n.signalId + 1]
                    if c.turn == "right" and sig.allowRightOnRed then
                        return "stop_then_yield"
                    end
                    return "stop"
                end
            end
            do
                return "stop"
            end
        until true
    end
    repeat
        local ____switch28 = ap.sign
        local ____cond28 = ____switch28 == "stop"
        if ____cond28 then
            return n.control == "allway_stop" and "allway" or "stop_then_yield"
        end
        ____cond28 = ____cond28 or ____switch28 == "yield"
        if ____cond28 then
            return "yield"
        end
        do
            return #n.roads <= 2 and "go" or "yield"
        end
    until true
end
--- Seconds needed for `v` to travel `dist` from its current speed (accelerating with `a`, capped at vmax).
function ____exports.timeToCover(speed, dist, a, vmax)
    if dist <= 0 then
        return 0
    end
    local sp = math.max(0, speed)
    local vm = math.max(vmax, sp, 1)
    local tAcc = (vm - sp) / a
    local dAcc = sp * tAcc + 0.5 * a * tAcc * tAcc
    if dAcc >= dist then
        return (-sp + math.sqrt(sp * sp + 2 * a * dist)) / a
    end
    return tAcc + (dist - dAcc) / vm
end
--- Vehicles already inside the junction on conflicting paths that would still
-- be in a shared zone when we arrive. Applies regardless of priority (you
-- don't T-bone a red-light runner just because you have green).
local function committedBlock(ctx, c, me, sMe, speed, strict)
    do
        local k = 0
        while k < #c.conflicts do
            do
                local __continue33
                repeat
                    local z = c.conflicts[k + 1]
                    if z.s1 < sMe - me.halfLen then
                        __continue33 = true
                        break
                    end
                    local o = z.other
                    do
                        local i = 0
                        while i < o.occCount do
                            do
                                local __continue35
                                repeat
                                    local e = o.occ[i + 1]
                                    local occ = e.occ
                                    if occ.id == me.id then
                                        __continue35 = true
                                        break
                                    end
                                    local half = occ.length / 2
                                    if e.s - half > z.os1 then
                                        __continue35 = true
                                        break
                                    end
                                    local inside = e.s + half > z.os0
                                    if not inside and e.v < 0.5 then
                                        if not ____exports.hasPriority(ctx, o, c) then
                                            __continue35 = true
                                            break
                                        end
                                        if not strict or me.id < occ.id then
                                            __continue35 = true
                                            break
                                        end
                                    end
                                    local tClear = (z.os1 - (e.s - half)) / math.max(e.v, 1)
                                    local tEnter = math.max(0, z.s0 - sMe - me.halfLen) / math.max(speed, 3)
                                    if tClear > tEnter - 0.3 then
                                        return true
                                    end
                                    __continue35 = true
                                until true
                                if not __continue35 then
                                    break
                                end
                            end
                            i = i + 1
                        end
                    end
                    __continue33 = true
                until true
                if not __continue33 then
                    break
                end
            end
            k = k + 1
        end
    end
    return false
end
--- Gap acceptance against movements with priority: vehicles on, or heading for,
-- conflicting connectors that arrive within our critical gap.
local function priorityBlock(ctx, d, c, distToLine, gapScale)
    local me = d.v
    local a = 2
    local vmax = c.speedLimit
    do
        local k = 0
        while k < #c.conflicts do
            do
                local __continue43
                repeat
                    local z = c.conflicts[k + 1]
                    local o = z.other
                    if not ____exports.hasPriority(ctx, o, c) then
                        __continue43 = true
                        break
                    end
                    local tMe = ____exports.timeToCover(me.speed, distToLine + z.s1 + me.length, a, vmax)
                    local need = math.max(d.p.criticalGap * gapScale, tMe + 1.2)
                    do
                        local i = 0
                        while i < o.occCount do
                            do
                                local __continue45
                                repeat
                                    local e = o.occ[i + 1]
                                    if e.occ.id == me.id then
                                        __continue45 = true
                                        break
                                    end
                                    if e.s - e.occ.length / 2 > z.os1 then
                                        __continue45 = true
                                        break
                                    end
                                    local dist = z.os0 - e.s - e.occ.length / 2
                                    local t = dist <= 0 and 0 or dist / math.max(e.v, 0.5)
                                    if e.v < 0.5 and dist > 2 then
                                        __continue45 = true
                                        break
                                    end
                                    if t < need then
                                        return true
                                    end
                                    __continue45 = true
                                until true
                                if not __continue45 then
                                    break
                                end
                            end
                            i = i + 1
                        end
                    end
                    local lane = o.from
                    do
                        local i = 0
                        while i < lane.occCount do
                            do
                                local __continue50
                                repeat
                                    local e = lane.occ[i + 1]
                                    local occ = e.occ
                                    if occ.id == me.id then
                                        __continue50 = true
                                        break
                                    end
                                    if occ.intentConnector ~= o.id and occ.intentConnector ~= -1 then
                                        __continue50 = true
                                        break
                                    end
                                    if e.v < 1 then
                                        __continue50 = true
                                        break
                                    end
                                    local dist = lane.length - e.s - occ.length / 2 + z.os0
                                    local t = dist / math.max(e.v, 0.5)
                                    if t < need then
                                        return true
                                    end
                                    __continue50 = true
                                until true
                                if not __continue50 then
                                    break
                                end
                            end
                            i = i + 1
                        end
                    end
                    __continue43 = true
                until true
                if not __continue43 then
                    break
                end
            end
            k = k + 1
        end
    end
    return false
end
--- Enough room on the exit lane to clear the junction?
local function boxBlocked(c, me)
    local lane = c.to
    local minS = math.huge
    local minV = 0
    do
        local i = 0
        while i < lane.occCount do
            do
                local __continue56
                repeat
                    local e = lane.occ[i + 1]
                    if e.occ.id == me.id then
                        __continue56 = true
                        break
                    end
                    local tail = e.s - e.occ.length / 2
                    if tail < minS then
                        minS = tail
                        minV = e.v
                    end
                    __continue56 = true
                until true
                if not __continue56 then
                    break
                end
            end
            i = i + 1
        end
    end
    local onC = 0
    do
        local i = 0
        while i < c.occCount do
            local e = c.occ[i + 1]
            if e.occ.id ~= me.id then
                onC = onC + (e.occ.length + 2)
            end
            i = i + 1
        end
    end
    if minS == math.huge then
        return onC > c.length + 4
    end
    return minS - onC < me.length + 2.5 and minV < 3
end
--- Another car is already waiting inside the box on our movement (one left-turner at a time).
local function connectorHasWaiter(c, me)
    do
        local i = 0
        while i < c.occCount do
            local e = c.occ[i + 1]
            if e.occ.id ~= me.id and e.v < 1.5 then
                return true
            end
            i = i + 1
        end
    end
    return false
end
local function firstConflictStart(c)
    local s = c.length
    for ____, z in ipairs(c.conflicts) do
        if z.kind == "cross" and z.s0 < s then
            s = z.s0
        end
    end
    return s
end
--- All-way stop: is it our turn among vehicles waiting at conflicting approaches?
local function allWayTurn(ctx, d, c)
    local q = ctx:allWay(c.node.id)
    local mine = q.arrivals:get(d.v.id)
    if mine == nil then
        return false
    end
    for ____, z in ipairs(c.conflicts) do
        local lane = z.other.from
        do
            local i = 0
            while i < lane.occCount do
                do
                    local __continue72
                    repeat
                        local e = lane.occ[i + 1]
                        if e.occ.id == d.v.id then
                            __continue72 = true
                            break
                        end
                        local t = q.arrivals:get(e.occ.id)
                        if t == nil then
                            __continue72 = true
                            break
                        end
                        if e.occ.intentConnector ~= z.other.id and e.occ.intentConnector ~= -1 then
                            __continue72 = true
                            break
                        end
                        if t < mine - 0.05 or math.abs(t - mine) <= 0.05 and e.occ.id < d.v.id then
                            return false
                        end
                        __continue72 = true
                    until true
                    if not __continue72 then
                        break
                    end
                end
                i = i + 1
            end
        end
    end
    return true
end
--- Distance (from our front bumper) at which we must stop for the junction
-- reached via connector `c`, or Infinity to proceed. Called while on the lane
-- leading into c.
function ____exports.approachStopDistance(ctx, d, lane, c, dt)
    local me = d.v
    if d.js.laneId ~= lane.id then
        d.js:reset(lane.id)
    end
    local distToLine = lane.length - me.s - me.halfLen
    local sp = math.max(0, me.speed)
    local range = math.max(35, sp * sp / 3 + 20)
    if distToLine > range then
        return math.huge
    end
    local mode = ____exports.entryMode(ctx, d, c, distToLine)
    local stopped = sp < 0.4 and distToLine < 3
    local rainScale = 1 + ctx.env.wetness * 0.25 + (ctx.env.dark and 0.1 or 0)
    local impatience = clamp(d.js.waitTime / 30, 0, 0.5)
    local gapScale = rainScale * (d.raging and 0.6 or 1) * (1 - impatience)
    if mode == "stop" then
        d.js.reason = "red"
        return distToLine
    end
    if mode == "stop_then_yield" or mode == "allway" then
        if not d.js.hasStopped then
            local rolling = d.p.lawfulness < 0.35 and sp < 3 and distToLine < 2
            if stopped or rolling then
                d.js.hasStopped = true
            else
                d.js.reason = "stop sign"
                return distToLine
            end
        end
        if mode == "allway" then
            local q = ctx:allWay(c.node.id)
            if not d.js.queued then
                q.arrivals:set(me.id, ctx.time)
                d.js.queued = true
            end
            if not allWayTurn(ctx, d, c) and d.js.waitTime < 12 then
                local ____d_js_0, ____waitTime_1 = d.js, "waitTime"
                ____d_js_0[____waitTime_1] = ____d_js_0[____waitTime_1] + dt
                d.js.reason = "all-way order"
                return distToLine
            end
        else
            mode = "yield"
        end
    end
    if committedBlock(
        ctx,
        c,
        me,
        -me.halfLen - distToLine,
        math.max(sp, 3),
        d.js.waitTime < 15
    ) then
        local ____d_js_2, ____waitTime_3 = d.js, "waitTime"
        ____d_js_2[____waitTime_3] = ____d_js_2[____waitTime_3] + dt
        d.js.reason = "junction occupied"
        return distToLine
    end
    if mode == "yield" or mode == "allway" then
        if priorityBlock(
            ctx,
            d,
            c,
            distToLine,
            gapScale
        ) then
            local ind = d.js.lastIndication
            if c.turn == "left" and (ind == "permissive" or ind == "flash_yellow") and not boxBlocked(c, me) and not connectorHasWaiter(c, me) then
                d.js.reason = "waiting in box"
                return distToLine + firstConflictStart(c) - 1
            end
            local ____d_js_4, ____waitTime_5 = d.js, "waitTime"
            ____d_js_4[____waitTime_5] = ____d_js_4[____waitTime_5] + dt
            d.js.reason = "yield"
            return distToLine
        end
    end
    if d.js.laneId == lane.id and not d.js.boxIgnore and d.js.waitTime == 0 and d.p.lawfulness < 0.3 and d:roll() < 0.3 then
        d.js.boxIgnore = true
    end
    if not d.js.boxIgnore and boxBlocked(c, me) then
        d.js.reason = "exit blocked"
        return distToLine
    end
    d.js.reason = ""
    d.js.waitTime = 0
    return math.huge
end
--- While on a connector: distance to stop before a conflict zone that is still
-- occupied (or contested by a priority movement for permissive lefts).
function ____exports.connectorStopDistance(ctx, d, c)
    local me = d.v
    local front = me.s + me.halfLen
    local sp = math.max(0, me.speed)
    local stopAt = math.huge
    for ____, z in ipairs(c.conflicts) do
        do
            local __continue97
            repeat
                if z.s0 < front - 0.5 then
                    __continue97 = true
                    break
                end
                local distToZone = z.s0 - front - 0.5
                local tArrive = distToZone / math.max(sp, 2)
                local o = z.other
                local blocked = false
                do
                    local i = 0
                    while i < o.occCount and not blocked do
                        do
                            local __continue99
                            repeat
                                local e = o.occ[i + 1]
                                if e.occ.id == me.id then
                                    __continue99 = true
                                    break
                                end
                                local half = e.occ.length / 2
                                if e.s - half > z.os1 then
                                    __continue99 = true
                                    break
                                end
                                local inside = e.s + half > z.os0
                                if not inside and e.v < 0.5 and me.id < e.occ.id then
                                    __continue99 = true
                                    break
                                end
                                if not inside and not ____exports.hasPriority(ctx, o, c) and e.v < 3 then
                                    __continue99 = true
                                    break
                                end
                                local tClear = (z.os1 - (e.s - half)) / math.max(e.v, 1)
                                local tThem = inside and 0 or math.max(0, z.os0 - e.s - half) / math.max(e.v, 0.5)
                                if inside or ____exports.hasPriority(ctx, o, c) and tThem < tArrive + 2 or tClear > tArrive - 0.2 then
                                    blocked = true
                                end
                                __continue99 = true
                            until true
                            if not __continue99 then
                                break
                            end
                        end
                        i = i + 1
                    end
                end
                if not blocked and c.turn == "left" and ____exports.hasPriority(ctx, o, c) then
                    local lane = o.from
                    local tMe = ____exports.timeToCover(sp, distToZone + (z.s1 - z.s0) + me.length, 2, c.speedLimit)
                    do
                        local i = 0
                        while i < lane.occCount do
                            do
                                local __continue106
                                repeat
                                    local e = lane.occ[i + 1]
                                    if e.occ.id == me.id or e.v < 1 then
                                        __continue106 = true
                                        break
                                    end
                                    if e.occ.intentConnector ~= o.id and e.occ.intentConnector ~= -1 then
                                        __continue106 = true
                                        break
                                    end
                                    local t = (lane.length - e.s - e.occ.length / 2 + z.os0) / math.max(e.v, 0.5)
                                    if t < tMe + 1 then
                                        blocked = true
                                        break
                                    end
                                    __continue106 = true
                                until true
                                if not __continue106 then
                                    break
                                end
                            end
                            i = i + 1
                        end
                    end
                end
                if blocked then
                    stopAt = math.min(stopAt, distToZone)
                    break
                end
                __continue97 = true
            until true
            if not __continue97 then
                break
            end
        end
    end
    return stopAt
end
function ____exports.nodeOf(c)
    return c.node
end
return ____exports
 end,
["src.core.ai.speech"] = function(...) 
--[[ Generated with https://github.com/TypeScriptToLua/TypeScriptToLua ]]
local ____exports = {}
local BY_OFFENSE = {
    cut_off = {
        "Use your signal!",
        "Whoa! Watch it!",
        "Are you kidding me?!",
        "Nice merge, genius.",
        "Hey! I was there!"
    },
    tailgating = {"Back off!", "Get off my bumper!", "Want to sit in my trunk?", "Personal space, choom!"},
    honked_at = {"Yeah, yeah, I hear you.", "Honk again, I dare you.", "Relax!", "What's your problem?"},
    long_honk = {"Okay, OKAY!", "Lay off the horn!", "You want a piece of me?!"},
    high_beams = {"Dim those lights!", "I'm blind here!", "Flash me again..."},
    blocked = {"Move it!", "Light's green, wake up!", "Any day now...", "GO!"},
    collision = {"You hit my car!", "Are you insane?!", "My paint job!", "You're paying for that!"},
    near_miss = {"Jesus! That was close!", "Watch where you're going!", "Almost killed me!"},
    rude_gesture = {"Oh, you're dead.", "Same to you, pal!", "Real mature!", "Come say that to my face!"},
    brake_check = {"What are you doing?!", "Psycho!", "Brake-checking me?!"},
    slow_driver = {"Gas pedal's on the right!", "Speed it up!", "Come ON..."},
    long_wait = {"This traffic...", "Unbelievable.", "I'm gonna be late."}
}
local BY_MOOD = {calm = {"...", "Easy does it."}, irritated = {"Ugh.", "Seriously?", "Come on.", "Every single day..."}, angry = {"Learn to drive!", "Get off the road!", "Idiot!", "Unreal!"}, furious = {"THAT'S IT!", "You're gonna regret that!", "Pull over, coward!", "I'm coming for you!"}}
____exports.APOLOGY_ACCEPTED = {
    "Alright, alright.",
    "No worries.",
    "Okay, fine.",
    "Just be careful.",
    "Apology accepted."
}
____exports.APOLOGY_REJECTED = {"Sorry won't fix it!", "Yeah, you'd better be sorry!", "Too late for that!"}
____exports.POLICE_LINES = {"Pull over!", "Stop the vehicle!", "License and registration.", "Pull to the right!"}
function ____exports.lineFor(rng, mood, kind)
    if kind ~= nil and rng:chance(0.7) then
        local l = BY_OFFENSE[kind]
        if l ~= nil then
            return rng:pick(l)
        end
    end
    return rng:pick(BY_MOOD[mood])
end
return ____exports
 end,
["src.core.ai.driver"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local __TS__New = ____lualib.__TS__New
local __TS__ArrayIndexOf = ____lualib.__TS__ArrayIndexOf
local ____exports = {}
local ____scalar = require("src.core.math.scalar")
local angleDiff = ____scalar.angleDiff
local clamp = ____scalar.clamp
local DEG = ____scalar.DEG
local lerp = ____scalar.lerp
local mod = ____scalar.mod
local ____rng = require("src.core.math.rng")
local Rng = ____rng.Rng
local ____network = require("src.core.road.network")
local addOccupant = ____network.addOccupant
local ____specs = require("src.core.vehicle.specs")
local maxAccelAt = ____specs.maxAccelAt
local ____control = require("src.core.ai.control")
local brakeToStop = ____control.brakeToStop
local driveToward = ____control.driveToward
local ____emotion = require("src.core.ai.emotion")
local Emotion = ____emotion.Emotion
local ____idm = require("src.core.ai.idm")
local idmAccel = ____idm.idmAccel
local idmFree = ____idm.idmFree
local ____intersection = require("src.core.ai.intersection")
local approachStopDistance = ____intersection.approachStopDistance
local connectorStopDistance = ____intersection.connectorStopDistance
local JunctionState = ____intersection.JunctionState
local ____speech = require("src.core.ai.speech")
local lineFor = ____speech.lineFor
--- Anger vehicles in front of a honking car.
function ____exports.hornVictims(ctx, honker, long)
    local hc = math.cos(honker.heading)
    local hs = math.sin(honker.heading)
    for ____, o in ipairs(ctx.vehicles) do
        do
            local __continue291
            repeat
                if o == honker or not o.alive then
                    __continue291 = true
                    break
                end
                local dx = o.x - honker.x
                local dy = o.y - honker.y
                local fwd = dx * hc + dy * hs
                if fwd < 0 or fwd > 30 then
                    __continue291 = true
                    break
                end
                local side = math.abs(-dx * hs + dy * hc)
                if side > 4 + fwd * 0.2 then
                    __continue291 = true
                    break
                end
                ctx:offend(honker, o, long and "long_honk" or "honked_at", 1)
                __continue291 = true
            until true
            if not __continue291 then
                break
            end
        end
    end
end
local params = {
    v0 = 10,
    T = 1.4,
    a = 1.5,
    b = 2,
    s0 = 2
}
local stopParams = {
    v0 = 10,
    T = 0.2,
    a = 1.5,
    b = 2,
    s0 = 0.6
}
--- Brain of an AI vehicle. In lane mode it follows its route with IDM car
-- following, MOBIL-style lane changes and the junction rules in
-- intersection.ts. Emotions (emotion.ts) modulate parameters and trigger
-- horn/high-beam use, speech and road-rage episodes; physics mode handles
-- crashes, recovery and chases.
____exports.AIDriver = __TS__Class()
local AIDriver = ____exports.AIDriver
AIDriver.name = "AIDriver"
function AIDriver.prototype.____constructor(self, v, p, seed)
    self.v = v
    self.p = p
    self.js = __TS__New(JunctionState)
    self.route = {}
    self.ri = 0
    self.next = nil
    self.state = "drive"
    self.emergency = false
    self.rage = nil
    self.leader = {
        id = -1,
        gap = math.huge,
        speed = 0,
        phantom = false,
        seg = nil,
        s = 0
    }
    self.prevLeaderId = -1
    self.prevLeaderGap = math.huge
    self.stopDist = math.huge
    self.lcIntent = nil
    self.lcIntentTime = 0
    self.shadowLane = nil
    self.latTarget = 0
    self.hornCooldown = 0
    self.hornHold = 0
    self.flashTimer = 0
    self.flashCount = 0
    self.speechCooldown = 0
    self.blockedTime = 0
    self.slowTime = 0
    self.distraction = 0
    self.distractCheck = 0
    self.reactTimer = 0
    self.yieldEmergency = 0
    self.lastMood = "calm"
    self.lastAccel = 0
    self.crashTimer = 0
    self.recoverLane = nil
    self.wanderTimer = 0
    self.stuckTime = 0
    self.followTarget = -1
    self.followGap = 10
    self.headFor = nil
    self.mood = "calm"
    self.raging = false
    self.signalDecided = -1
    self.willSignal = true
    self.emo = __TS__New(Emotion, p)
    self.rng = __TS__New(Rng, seed)
    self.lcTimer = self.rng:range(0.5, 2.5)
    self.distractCheck = self.rng:range(0, 1)
end
function AIDriver.prototype.syncFlags(self)
    self.mood = self.emo.mood
    self.raging = self.rage ~= nil
end
function AIDriver.prototype.roll(self)
    return self.rng:next()
end
function AIDriver.prototype.status(self)
    if self.state ~= "drive" then
        return self.state
    end
    if self.rage ~= nil then
        return "RAGE:" .. self.rage.behavior
    end
    if self.distraction > 0 then
        return "distracted"
    end
    if self.yieldEmergency > 0 then
        return "yield-emergency"
    end
    if self.js.reason ~= "" and self.stopDist < 40 then
        return self.js.reason
    end
    if self.lcIntent ~= nil then
        return "changing lanes"
    end
    return self.v.speed < 0.3 and "stopped" or "driving"
end
function AIDriver.prototype.nextRoad(self, idx)
    return idx < #self.route and self.route[idx + 1] or nil
end
function AIDriver.prototype.newDestination(self, ctx, lane)
    local net = ctx.net
    do
        local attempt = 0
        while attempt < 6 do
            do
                local __continue14
                repeat
                    local goal = net.lanes[math.floor(self.rng:next() * #net.lanes) + 1]
                    if goal.road == lane.road then
                        __continue14 = true
                        break
                    end
                    local r = ctx.router:route(lane, goal.road, goal.forward)
                    if r ~= nil and #r > 0 then
                        self.route = r
                        self.ri = 0
                        self.next = self:connectorFrom(lane)
                        return
                    end
                    __continue14 = true
                until true
                if not __continue14 then
                    break
                end
            end
            attempt = attempt + 1
        end
    end
    self.route = {}
    self.ri = 0
    self.next = self:connectorFrom(lane)
end
function AIDriver.prototype.routeTo(self, ctx, road, forward)
    local lane = self:currentLane()
    if lane == nil then
        return false
    end
    if lane.road == road then
        return true
    end
    local r = ctx.router:route(lane, road, forward)
    if r == nil then
        return false
    end
    self.route = r
    self.ri = 0
    self.next = self:connectorFrom(lane)
    return true
end
function AIDriver.prototype.currentLane(self)
    local seg = self.v.seg
    if seg == nil then
        return nil
    end
    return seg.kind == "lane" and seg or seg.to
end
function AIDriver.prototype.connectorFrom(self, lane)
    local step = self:nextRoad(self.ri)
    if step ~= nil then
        local c = lane:connectorTo(step.road)
        if c ~= nil then
            return c
        end
    end
    if #lane.outgoing == 0 then
        return nil
    end
    if step == nil then
        return lane.outgoing[math.floor(self.rng:next() * #lane.outgoing) + 1]
    end
    for ____, c in ipairs(lane.outgoing) do
        if c.turn == "straight" then
            return c
        end
    end
    return lane.outgoing[1]
end
function AIDriver.prototype.routeLanes(self, lane)
    local step = self:nextRoad(self.ri)
    local out = {}
    if step == nil then
        return out
    end
    for ____, l in ipairs(lane:siblings()) do
        if l:connectorTo(step.road) ~= nil then
            out[#out + 1] = l
        end
    end
    return out
end
function AIDriver.prototype.scanLeader(self, ctx)
    local v = self.v
    local L = self.leader
    L.id = -1
    L.gap = math.huge
    L.speed = 0
    L.phantom = false
    L.seg = nil
    local seg0 = v.seg
    if seg0 == nil then
        return
    end
    local look = math.max(55, v.speed * 4.5 + 25)
    local base = -v.s
    local seg = seg0
    local depth = 0
    local bestD = math.huge
    local nextConn = self.next
    while seg ~= nil and depth < 5 do
        do
            local i = 0
            while i < seg.occCount do
                do
                    local __continue39
                    repeat
                        local e = seg.occ[i + 1]
                        if e.occ.id == v.id then
                            __continue39 = true
                            break
                        end
                        if depth == 0 and e.s <= v.s + 0.05 then
                            __continue39 = true
                            break
                        end
                        if depth == 0 and math.abs(e.lat - v.lat) > (e.occ.width + v.width) / 2 + 0.25 then
                            __continue39 = true
                            break
                        end
                        local d = base + e.s - (e.occ.length + v.length) / 2
                        if d < bestD then
                            bestD = d
                            L.id = e.occ.id
                            L.speed = e.v
                            L.phantom = e.phantom
                            L.seg = seg
                            L.s = e.s
                        end
                        __continue39 = true
                    until true
                    if not __continue39 then
                        break
                    end
                end
                i = i + 1
            end
        end
        if seg.kind == "connector" then
            local conn = seg
            local sib = conn.siblings
            do
                local k = 0
                while k < #sib do
                    local o = sib[k + 1]
                    local diverge = conn.siblingDiverge[k + 1]
                    do
                        local i = 0
                        while i < o.occCount do
                            do
                                local __continue46
                                repeat
                                    local e = o.occ[i + 1]
                                    if e.occ.id == v.id or e.s - e.occ.length / 2 > diverge then
                                        __continue46 = true
                                        break
                                    end
                                    if depth == 0 and e.s <= v.s then
                                        __continue46 = true
                                        break
                                    end
                                    local d = base + e.s - (e.occ.length + v.length) / 2
                                    if d < bestD then
                                        bestD = d
                                        L.id = e.occ.id
                                        L.speed = e.v
                                        L.phantom = e.phantom
                                        L.seg = o
                                        L.s = e.s
                                    end
                                    __continue46 = true
                                until true
                                if not __continue46 then
                                    break
                                end
                            end
                            i = i + 1
                        end
                    end
                    k = k + 1
                end
            end
        end
        if depth == 0 and self.shadowLane ~= nil and seg.kind == "lane" then
            local sh = self.shadowLane
            local sMe = v.s * sh.length / seg.length
            do
                local i = 0
                while i < sh.occCount do
                    do
                        local __continue51
                        repeat
                            local e = sh.occ[i + 1]
                            if e.occ.id == v.id or e.s <= sMe then
                                __continue51 = true
                                break
                            end
                            local d = e.s - sMe - (e.occ.length + v.length) / 2
                            if d < bestD then
                                bestD = d
                                L.id = e.occ.id
                                L.speed = e.v
                                L.phantom = e.phantom
                                L.seg = sh
                                L.s = e.s
                            end
                            __continue51 = true
                        until true
                        if not __continue51 then
                            break
                        end
                    end
                    i = i + 1
                end
            end
        end
        if bestD < math.huge then
            break
        end
        base = base + seg.length
        if base > look then
            break
        end
        if seg.kind == "lane" then
            local ____temp_1
            if depth == 0 then
                ____temp_1 = nextConn
            else
                local ____temp_0
                if #seg.outgoing > 0 then
                    ____temp_0 = self:peekConnector(seg, depth)
                else
                    ____temp_0 = nil
                end
                ____temp_1 = ____temp_0
            end
            local c = ____temp_1
            seg = c
        else
            local lane = seg.to
            seg = lane
            nextConn = nil
        end
        depth = depth + 1
    end
    L.gap = bestD
end
function AIDriver.prototype.peekConnector(self, lane, depth)
    local idx = self.ri + (self.v.seg ~= nil and self.v.seg.kind == "connector" and 0 or 1) + math.floor((depth - 1) / 2)
    local step = self:nextRoad(idx)
    if step ~= nil then
        local c = lane:connectorTo(step.road)
        if c ~= nil then
            return c
        end
    end
    for ____, c in ipairs(lane.outgoing) do
        if c.turn == "straight" then
            return c
        end
    end
    return #lane.outgoing > 0 and lane.outgoing[1] or nil
end
function AIDriver.prototype.update(self, dt, ctx)
    local v = self.v
    self.hornCooldown = self.hornCooldown - dt
    self.speechCooldown = self.speechCooldown - dt
    self:updateHornAndFlash(dt)
    self.emo:update(
        dt,
        ctx.time,
        self:congestion()
    )
    self:syncFlags()
    v.lights.dark = ctx.env.dark
    if v.wrecked then
        self.state = "crashed"
        v.lights.hazard = true
    end
    if v.mode == "physics" then
        self:physicsUpdate(dt, ctx)
        return
    end
    self:laneUpdate(dt, ctx)
end
function AIDriver.prototype.congestion(self)
    local lim = self.v.seg ~= nil and self.v.seg.speedLimit or 10
    return clamp(
        1 - self.v.speed / math.max(lim, 1),
        0,
        1
    ) * (self.leader.gap < 15 and 1 or 0.3)
end
function AIDriver.prototype.laneUpdate(self, dt, ctx)
    local v = self.v
    local seg = v.seg
    if seg == nil then
        return
    end
    local p = self.p
    if seg.kind == "lane" and (self.next == nil or self.next.from ~= seg) then
        self.next = self:connectorFrom(seg)
    end
    v.intentConnector = seg.kind == "lane" and self.next ~= nil and self.next.id or (seg.kind == "connector" and seg.id or -1)
    self:scanLeader(ctx)
    self:detectOffenses(dt, ctx)
    self:checkEmergencyBehind(dt, ctx)
    self:updateRage(dt, ctx)
    self:syncFlags()
    if v.mode ~= "lane" then
        return
    end
    local mood = self.emo.mood
    local moodSpeed = mood == "angry" and 1.1 or (mood == "furious" and 1.2 or (mood == "irritated" and 1.04 or 1))
    local moodT = mood == "angry" and 0.7 or (mood == "furious" and 0.55 or (mood == "irritated" and 0.88 or 1))
    local wet = ctx.env.wetness
    local vis = ctx.env.visibility
    local weatherSpeed = (1 - wet * 0.15) * clamp(vis / 250, 0.55, 1)
    params.v0 = seg.speedLimit * p.speedFactor * moodSpeed * weatherSpeed
    if self.emergency then
        params.v0 = seg.speedLimit * 1.45
    end
    params.T = p.headway * moodT * (1 + wet * 0.3)
    params.a = math.min(
        maxAccelAt(v.spec, v.speed),
        2.6
    ) * p.accelFactor
    params.b = p.comfortDecel
    params.s0 = p.minGap
    local rage = self.rage
    if rage ~= nil and rage.target == self.leader.id and rage.behavior == "tailgate" then
        params.T = 0.3
        params.s0 = 1
        params.v0 = params.v0 * 1.3
    end
    local acc = idmFree(v.speed, params)
    if self.leader.gap < math.huge then
        acc = idmAccel(v.speed, self.leader.gap, v.speed - self.leader.speed, params)
    end
    if seg.kind == "lane" and self.next ~= nil then
        local vc = self.next.speedLimit * lerp(0.9, 1.15, p.aggression)
        local dc = seg.length - v.s - v.halfLen
        if v.speed > vc then
            acc = math.min(
                acc,
                -((v.speed * v.speed - vc * vc) / (2 * math.max(dc, 2))) * 1.15
            )
        end
    elseif seg.kind == "connector" then
        local vc = seg.speedLimit * lerp(0.95, 1.15, p.aggression)
        if v.speed > vc then
            acc = math.min(acc, -1.5)
        end
    end
    self.stopDist = math.huge
    if seg.kind == "lane" and self.next ~= nil and not self:ragingThroughLights() then
        self.stopDist = approachStopDistance(
            ctx,
            self,
            seg,
            self.next,
            dt
        )
        if self.emergency and self.next.node.control == "signal" and seg.length - v.s < 120 then
            ctx.signals[self.next.node.signalId + 1]:requestPreempt(self.next.approach)
        end
    elseif seg.kind == "connector" then
        self.stopDist = connectorStopDistance(ctx, self, seg)
    end
    if seg.kind == "lane" and self.next == nil then
        self.stopDist = math.min(self.stopDist, seg.length - v.s - v.halfLen)
    end
    if self.stopDist < math.huge then
        stopParams.a = params.a
        stopParams.b = params.b
        stopParams.v0 = params.v0
        acc = math.min(
            acc,
            idmAccel(v.speed, self.stopDist, v.speed, stopParams)
        )
    end
    if self.yieldEmergency > 0 then
        local lane = seg.kind == "lane" and seg or nil
        if lane ~= nil and lane.right == nil then
            self.latTarget = -(lane.width * 0.42)
        end
        acc = math.min(acc, v.speed > 2 and -2.5 or -0.5)
    elseif self.emergency and self.leader.gap < 30 and self.leader.speed < 3 then
        self.latTarget = 1.2
    else
        self.latTarget = 0
    end
    if rage ~= nil and rage.brakeCheck > 0 then
        acc = math.min(acc, -7)
    end
    acc = self:humanFactors(dt, acc)
    local maxBrake = 8.5 * ctx.env.grip * v.spec.grip
    acc = clamp(acc, -maxBrake, params.a * 1.05)
    self.lastAccel = acc
    self:integrate(dt, acc, ctx)
    self:laneChanges(dt, ctx)
    self:lateral(dt)
    self:updateLights(dt, ctx)
    self:updateWaiting(dt, ctx)
end
function AIDriver.prototype.ragingThroughLights(self)
    return self.rage ~= nil and (self.rage.behavior == "tailgate" or self.rage.behavior == "chase") and self.emo.anger > 85 and self.p.lawfulness < 0.3
end
function AIDriver.prototype.humanFactors(self, dt, acc)
    local v = self.v
    local p = self.p
    if v.speed < 0.15 and acc > 0.2 then
        self.reactTimer = self.reactTimer + dt
        local reaction = lerp(1.4, 0.45, p.skill) + (self.distraction > 0 and 2.5 or 0)
        if self.reactTimer < reaction then
            acc = 0
        end
    elseif v.speed > 0.5 then
        self.reactTimer = 0
    end
    self.distractCheck = self.distractCheck - dt
    if self.distractCheck <= 0 then
        self.distractCheck = 1
        if self.distraction <= 0 and self.rng:chance((1 - p.attentiveness) * 0.025) then
            self.distraction = self.rng:range(1.2, 3.5)
        end
    end
    if self.distraction > 0 then
        self.distraction = self.distraction - dt
        local ttc = self.leader.gap < math.huge and v.speed > self.leader.speed and self.leader.gap / (v.speed - self.leader.speed) or math.huge
        local stopTtc = self.stopDist < math.huge and v.speed > 0.5 and self.stopDist / v.speed or math.huge
        if ttc < 1.3 or stopTtc < 1 then
            self.distraction = 0
            return math.min(acc, -8)
        end
        return math.min(
            self.lastAccel,
            math.max(acc, self.lastAccel - 0.2)
        )
    end
    return acc
end
function AIDriver.prototype.integrate(self, dt, acc, ctx)
    local v = self.v
    local sp = v.speed + acc * dt
    if sp < 0 then
        sp = 0
    end
    v.accel = (sp - v.speed) / dt
    v.speed = sp
    v.s = v.s + sp * dt
    v.odometer = v.odometer + sp * dt
    v.wheelSpin = v.wheelSpin + sp * dt / v.spec.wheelRadius
    local guard = 0
    while v.seg ~= nil and v.s > v.seg.length and guard < 4 do
        guard = guard + 1
        local seg = v.seg
        local over = v.s - seg.length
        if seg.kind == "lane" then
            local c = self.next
            if c == nil or c.from ~= seg then
                c = self:connectorFrom(seg)
            end
            if c == nil then
                v.s = seg.length
                v.speed = 0
                break
            end
            if self.js.queued then
                ctx:allWay(c.node.id).arrivals:delete(v.id)
            end
            self.js.queued = false
            v.prevSeg = seg
            v.seg = c
            v.s = over
            self.shadowLane = nil
        else
            local lane = seg.to
            v.prevSeg = seg
            v.seg = lane
            v.s = over
            local step = self:nextRoad(self.ri)
            if step ~= nil and step.road == lane.road then
                self.ri = self.ri + 1
            elseif step ~= nil then
                self.route = {}
            end
            if self.headFor == nil and self.ri >= #self.route then
                self:newDestination(ctx, lane)
            end
            self.next = self:connectorFrom(lane)
            self.js:reset(lane.id)
            if self.lcIntent ~= nil then
                self.lcIntent = nil
                v.lights:setIndicator("off")
            end
        end
    end
    v:syncLanePose()
    if v.seg ~= nil then
        v.wheelAngle = clamp(
            v.seg.path:curvatureAt(v.s + 2, 2) * v.spec.wheelbase * 1.1 + self.v.latVel * 0.05,
            -0.6,
            0.6
        )
    end
end
function AIDriver.prototype.laneChanges(self, dt, ctx)
    local v = self.v
    local seg = v.seg
    if seg == nil or seg.kind ~= "lane" then
        return
    end
    local lane = seg
    if math.abs(v.lat) > 0.6 then
        return
    end
    local toEnd = lane.length - v.s
    if self.lcIntent ~= nil then
        self.lcIntentTime = self.lcIntentTime + dt
        local signalTime = lerp(0.6, 2.2, self.p.signalDiscipline)
        local urgency = clamp(1 - (toEnd - 20) / 80, 0, 1)
        if self.lcIntentTime >= signalTime * (1 - urgency * 0.7) then
            local bSafe = lerp(2.5, 5, self.p.aggression) + urgency * 3 + (self.raging and 2 or 0)
            if self:tryLaneChange(ctx, lane, self.lcIntent, bSafe) then
                return
            end
        end
        if self.lcIntentTime > 12 then
            self.lcIntent = nil
            v.lights:setIndicator("off")
        end
        return
    end
    self.lcTimer = self.lcTimer - dt
    if self.lcTimer > 0 then
        return
    end
    self.lcTimer = self.rng:range(0.8, 1.8)
    if toEnd < 12 or v.s < 4 then
        return
    end
    local good = self:routeLanes(lane)
    if #good > 0 and __TS__ArrayIndexOf(good, lane) < 0 then
        local ____temp_2
        if good[1].index < lane.index then
            ____temp_2 = lane.left
        else
            ____temp_2 = lane.right
        end
        local dir = ____temp_2
        if dir ~= nil then
            self:intend(dir)
        end
        return
    end
    if self.yieldEmergency > 0 and lane.right ~= nil then
        self:intend(lane.right)
        return
    end
    if v.speed < 1 and self.leader.gap > 8 then
        return
    end
    local aOld = self.leader.gap < math.huge and idmAccel(v.speed, self.leader.gap, v.speed - self.leader.speed, params) or idmFree(v.speed, params)
    local best = nil
    local bestGain = 0
    local cands = {lane.left, lane.right}
    for ____, cand in ipairs(cands) do
        do
            local __continue124
            repeat
                if cand == nil then
                    __continue124 = true
                    break
                end
                if #good > 0 and __TS__ArrayIndexOf(good, cand) < 0 and toEnd < 150 then
                    __continue124 = true
                    break
                end
                local g = self:evaluateLane(ctx, lane, cand)
                if g == nil then
                    __continue124 = true
                    break
                end
                local bias = cand == lane.right and 0.12 or -0.05
                local politeness = self.raging and 0 or self.p.politeness
                local gain = g.aNew - aOld + politeness * g.followerDelta + bias
                local threshold = lerp(0.45, 0.1, self.p.aggression)
                if gain > threshold and gain > bestGain then
                    bestGain = gain
                    best = cand
                end
                __continue124 = true
            until true
            if not __continue124 then
                break
            end
        end
    end
    if best ~= nil then
        self:intend(best)
    end
end
function AIDriver.prototype.intend(self, target)
    self.lcIntent = target
    self.lcIntentTime = 0
    local signals = self.rng:chance(self.p.signalDiscipline)
    if signals then
        self.v.lights:setIndicator(target == self.v.seg.left and "left" or "right")
    end
end
function AIDriver.prototype.evaluateLane(self, ctx, lane, cand)
    local v = self.v
    local sT = v.s * cand.length / lane.length
    local lead = nil
    local fol = nil
    do
        local i = 0
        while i < cand.occCount do
            do
                local __continue134
                repeat
                    local e = cand.occ[i + 1]
                    if e.occ.id == v.id then
                        __continue134 = true
                        break
                    end
                    if e.s >= sT then
                        if lead == nil or e.s < lead.s then
                            lead = {s = e.s, v = e.v, len = e.occ.length}
                        end
                    elseif fol == nil or e.s > fol.s then
                        fol = {s = e.s, v = e.v, len = e.occ.length, id = e.occ.id}
                    end
                    __continue134 = true
                until true
                if not __continue134 then
                    break
                end
            end
            i = i + 1
        end
    end
    if fol == nil and sT < 30 then
        for ____, c in ipairs(cand.incoming) do
            do
                local i = 0
                while i < c.occCount do
                    do
                        local __continue141
                        repeat
                            local e = c.occ[i + 1]
                            if e.occ.id == v.id then
                                __continue141 = true
                                break
                            end
                            local s = e.s - c.length
                            if fol == nil or s > fol.s then
                                fol = {s = s, v = e.v, len = e.occ.length, id = e.occ.id}
                            end
                            __continue141 = true
                        until true
                        if not __continue141 then
                            break
                        end
                    end
                    i = i + 1
                end
            end
        end
    end
    local gapAhead = lead == nil and math.huge or lead.s - sT - (lead.len + v.length) / 2
    local gapBehind = fol == nil and math.huge or sT - fol.s - (fol.len + v.length) / 2
    local minAhead = math.max(
        2.5,
        v.speed * lerp(0.6, 0.3, self.p.aggression)
    )
    local minBehind = fol == nil and 0 or math.max(
        2,
        fol.v * lerp(0.5, 0.25, self.p.aggression)
    )
    if gapAhead < minAhead or gapBehind < minBehind then
        return nil
    end
    local aNew = lead == nil and idmFree(v.speed, params) or idmAccel(v.speed, gapAhead, v.speed - lead.v, params)
    local followerDelta = 0
    local followerAcc = 0
    if fol ~= nil then
        local fp = {
            v0 = cand.speedLimit,
            T = 1.3,
            a = 1.5,
            b = 2,
            s0 = 2
        }
        local before = lead == nil and idmFree(fol.v, fp) or idmAccel(fol.v, lead.s - fol.s - (lead.len + fol.len) / 2, fol.v - lead.v, fp)
        followerAcc = idmAccel(fol.v, gapBehind, fol.v - v.speed, fp)
        followerDelta = followerAcc - before
    end
    return {
        aNew = aNew,
        followerDelta = followerDelta,
        followerAcc = followerAcc,
        follower = fol == nil and -1 or fol.id,
        gapAhead = gapAhead,
        gapBehind = gapBehind
    }
end
function AIDriver.prototype.tryLaneChange(self, ctx, lane, target, bSafe)
    local v = self.v
    if lane.length - v.s - v.halfLen < 10 and v.speed > 2 then
        return false
    end
    local ev = self:evaluateLane(ctx, lane, target)
    if ev == nil then
        return false
    end
    if ev.followerAcc < -bSafe then
        return false
    end
    local offset = target == lane.right and (lane.width + target.width) / 2 or -(lane.width + target.width) / 2
    local sT = v.s * target.length / lane.length
    self.shadowLane = lane
    v.seg = target
    v.s = sT
    v.lat = v.lat + offset
    addOccupant(
        target,
        v,
        sT,
        v.speed,
        false,
        v.lat
    )
    self.lcIntent = nil
    self.next = self:connectorFrom(target)
    self.js:reset(target.id)
    if ev.follower >= 0 and ev.followerAcc < -2.2 then
        local victim = ctx:vehicle(ev.follower)
        if victim ~= nil then
            ctx:offend(
                v,
                victim,
                "cut_off",
                clamp(-ev.followerAcc / 3, 0.5, 2)
            )
        end
    end
    return true
end
function AIDriver.prototype.lateral(self, dt)
    local v = self.v
    local err = self.latTarget - v.lat
    local maxV = lerp(0.9, 1.8, self.p.aggression) * (self.raging and 1.4 or 1)
    local want = clamp(err * 1.1, -maxV, maxV)
    v.latVel = v.latVel + clamp(want - v.latVel, -2.5 * dt, 2.5 * dt)
    v.lat = v.lat + v.latVel * dt
    if math.abs(err) < 0.03 and math.abs(v.latVel) < 0.05 then
        v.lat = self.latTarget
        v.latVel = 0
    end
    if self.shadowLane ~= nil and math.abs(v.lat) < v.width / 2 then
        self.shadowLane = nil
    end
    if self.lcIntent == nil and math.abs(v.lat) < 0.2 and self.v.seg ~= nil and self.v.seg.kind == "lane" then
        local lights = v.lights
        local turnSoon = self:turnSignalWanted()
        if turnSoon == "off" and lights.indicator ~= "off" and lights.indicatorTime > 1 then
            lights:setIndicator("off")
        end
    end
end
function AIDriver.prototype.turnSignalWanted(self)
    local v = self.v
    local seg = v.seg
    if seg == nil then
        return "off"
    end
    local c = nil
    local dist = 0
    if seg.kind == "lane" then
        c = self.next
        dist = seg.length - v.s
    else
        c = seg
    end
    if c == nil or dist > 45 then
        return "off"
    end
    if c.turn == "left" or c.turn == "uturn" then
        return "left"
    end
    if c.turn == "right" then
        return "right"
    end
    return "off"
end
function AIDriver.prototype.updateLights(self, dt, ctx)
    local v = self.v
    local lights = v.lights
    lights.brake = (self.lastAccel < -0.6 or v.speed < 0.2 and self.stopDist < 20 or v.speed < 0.2 and self.leader.gap < 8) and 1 or 0
    lights.reverse = false
    if self.state == "drive" and not v.wrecked then
        lights.hazard = self.yieldEmergency > 0 and v.speed < 1 and self.p.lawfulness > 0.8
    end
    if self.lcIntent == nil then
        local want = self:turnSignalWanted()
        local segId = v.seg ~= nil and v.seg.id or -1
        if self.signalDecided ~= segId then
            self.signalDecided = segId
            self.willSignal = self.rng:chance(self.p.signalDiscipline)
        end
        if want ~= "off" and self.willSignal then
            lights:setIndicator(want)
        elseif want == "off" and math.abs(v.lat) < 0.2 and lights.indicatorTime > 1.5 and v.seg ~= nil and v.seg.kind == "lane" then
            lights:setIndicator("off")
        end
    end
    lights:update(dt, v.wheelAngle)
    if self.emergency then
        lights.lightbar = true
        if lights.siren == "off" then
            lights.siren = "wail"
        end
    end
    if ctx.env.dark and lights.headlightSwitch == "off" and self.rng:chance(0.002) then
        lights.headlightSwitch = "auto"
    end
end
function AIDriver.prototype.updateHornAndFlash(self, dt)
    local l = self.v.lights
    if self.hornHold > 0 then
        self.hornHold = self.hornHold - dt
        l.horn = true
        if self.hornHold <= 0 then
            l.horn = false
        end
    end
    if self.flashTimer > 0 then
        self.flashTimer = self.flashTimer - dt
        local phase = mod(self.flashTimer, 0.5)
        l.flash = phase > 0.25
        if self.flashTimer <= 0 then
            l.flash = false
        end
    end
    local ____ = self.flashCount
end
function AIDriver.prototype.honk(self, ctx, long)
    if self.hornCooldown > 0 then
        return
    end
    self.hornHold = long and self.rng:range(1.2, 2.2) or self.rng:range(0.25, 0.45)
    self.hornCooldown = long and 4 or 2.2
    ctx.events:emit({type = "horn", vehicle = self.v.id, long = long})
    ____exports.hornVictims(ctx, self.v, long)
end
function AIDriver.prototype.flashHighBeams(self, ctx)
    if self.flashTimer > 0 then
        return
    end
    self.flashTimer = 1
    self.flashCount = self.flashCount + 1
    ctx.events:emit({type = "flash", vehicle = self.v.id})
    local ____temp_3
    if self.leader.id >= 0 then
        ____temp_3 = ctx:vehicle(self.leader.id)
    else
        ____temp_3 = nil
    end
    local leader = ____temp_3
    if leader ~= nil and self.leader.gap < 40 then
        ctx:offend(self.v, leader, "high_beams", 1)
    end
end
function AIDriver.prototype.say(self, ctx, text)
    if self.speechCooldown > 0 then
        return
    end
    self.speechCooldown = self.rng:range(5, 9)
    ctx.events:emit({type = "speech", vehicle = self.v.id, text = text, mood = self.emo.mood})
end
function AIDriver.prototype.onOffended(self, ctx, offender, kind)
    local mood = self.emo.mood
    local hornChance = self.p.hornHappiness * (mood == "calm" and 0.4 or 1)
    if kind ~= "honked_at" and kind ~= "long_honk" and self.rng:chance(hornChance) then
        self:honk(ctx, mood == "angry" or mood == "furious")
    end
    if mood ~= "calm" or kind == "collision" or kind == "near_miss" then
        if self.rng:chance(0.6) then
            self:say(
                ctx,
                lineFor(self.rng, mood, kind)
            )
        end
    end
    local ____ = offender
end
function AIDriver.prototype.detectOffenses(self, dt, ctx)
    local v = self.v
    local L = self.leader
    if L.id >= 0 and L.id ~= self.prevLeaderId and L.phantom and v.speed > 4 then
        local thw = L.gap / math.max(v.speed, 1)
        if thw < 0.75 and L.gap < self.prevLeaderGap - 3 then
            local off = ctx:vehicle(L.id)
            if off ~= nil then
                ctx:offend(
                    off,
                    v,
                    "cut_off",
                    clamp((0.9 - thw) * 2.5, 0.5, 2)
                )
            end
        end
    end
    self.prevLeaderId = L.id
    self.prevLeaderGap = L.gap
    local ____temp_4
    if L.id >= 0 then
        ____temp_4 = ctx:vehicle(L.id)
    else
        ____temp_4 = nil
    end
    local leaderVeh = ____temp_4
    if leaderVeh ~= nil and L.gap < 12 and L.speed < 0.3 and v.speed < 0.3 and not self:leaderHasReason(ctx, leaderVeh) then
        self.blockedTime = self.blockedTime + dt
        local limit = 2.5 + self.p.patience * 5
        if self.blockedTime > limit then
            ctx:offend(leaderVeh, v, "blocked", dt * 2)
            if self.hornCooldown <= 0 and self.rng:chance(0.4 + self.p.hornHappiness * 0.5) then
                self:honk(ctx, self.blockedTime > limit * 2 or self.emo.mood == "angry")
            end
        end
    else
        self.blockedTime = 0
    end
    local lim = v.seg ~= nil and v.seg.speedLimit or 10
    if leaderVeh ~= nil and L.gap < 25 and L.speed > 1 and L.speed < lim * 0.55 and self.stopDist == math.huge then
        self.slowTime = self.slowTime + dt
        if self.slowTime > 6 + self.p.patience * 10 then
            ctx:offend(leaderVeh, v, "slow_driver", dt)
        end
    else
        self.slowTime = 0
    end
end
function AIDriver.prototype.leaderHasReason(self, ctx, lead)
    local d = ctx:driverOf(lead)
    if d ~= nil then
        if d.stopDist < 8 then
            return true
        end
        if d.leader.gap < 8 and d.leader.speed < 0.5 then
            return true
        end
        if d.state ~= "drive" then
            local ____temp_5
            if d.state == "crashed" then
                ____temp_5 = false
            else
                ____temp_5 = true
            end
            return ____temp_5
        end
        return d.yieldEmergency > 0
    end
    local seg = self.leader.seg
    if seg ~= nil and seg.kind == "lane" then
        local lane = seg
        local toLine = lane.length - self.leader.s
        if toLine < 10 then
            local ap = lane.approach
            if ap ~= nil and (ap.sign == "stop" or ap.sign == "yield") then
                return true
            end
            if ap ~= nil and ap.sign == "signal" then
                local sig = ctx.signals[ap.node.signalId + 1]
                for ____, c in ipairs(lane.outgoing) do
                    if sig:indication(c) == "green" then
                        return self.blockedTime < 4
                    end
                end
                return true
            end
        end
        do
            local i = 0
            while i < lane.occCount do
                local e = lane.occ[i + 1]
                if e.occ.id ~= lead.id and e.s > self.leader.s and e.s - self.leader.s < 10 and e.v < 0.5 then
                    return true
                end
                i = i + 1
            end
        end
    end
    return false
end
function AIDriver.prototype.checkEmergencyBehind(self, dt, ctx)
    if self.yieldEmergency > 0 then
        self.yieldEmergency = self.yieldEmergency - dt
    end
    if self.emergency then
        return
    end
    local v = self.v
    local hc = math.cos(v.heading)
    local hs = math.sin(v.heading)
    for ____, e in ipairs(ctx.vehicles) do
        do
            local __continue218
            repeat
                if not e:sirenOn() or e == v or not e.alive then
                    __continue218 = true
                    break
                end
                local dx = e.x - v.x
                local dy = e.y - v.y
                local d2 = dx * dx + dy * dy
                if d2 > 80 * 80 then
                    __continue218 = true
                    break
                end
                local behind = dx * hc + dy * hs < -2
                local sameDir = math.abs(angleDiff(v.heading, e.heading)) < 50 * DEG
                if behind and sameDir then
                    self.yieldEmergency = 2.5
                    return
                end
                __continue218 = true
            until true
            if not __continue218 then
                break
            end
        end
    end
end
function AIDriver.prototype.updateWaiting(self, dt, ctx)
    local v = self.v
    if v.speed < 0.2 then
        self.stuckTime = self.stuckTime + dt
        if self.stuckTime > 30 then
            self.emo:offend(-1, "long_wait", dt * 2, ctx.time)
        end
    else
        self.stuckTime = 0
    end
    local t = self.emo.target
    if t >= 0 and (self.emo.mood == "angry" or self.emo.mood == "furious") and self.leader.id == t and self.leader.gap < 25 then
        if self.rng:chance(dt * 0.4 * (0.4 + self.p.hornHappiness)) then
            self:honk(ctx, true)
        end
        if self.rng:chance(dt * 0.3) then
            self:flashHighBeams(ctx)
        end
    end
end
function AIDriver.prototype.updateRage(self, dt, ctx)
    local v = self.v
    local emo = self.emo
    local rage = self.rage
    if rage == nil then
        if not emo:canRage() or emo.target < 0 or self.emergency then
            return
        end
        local tgt = ctx:vehicle(emo.target)
        if tgt == nil or not tgt.alive then
            return
        end
        local dist = math.sqrt((tgt.x - v.x) * (tgt.x - v.x) + (tgt.y - v.y) * (tgt.y - v.y))
        if dist > 70 then
            return
        end
        local police = ctx:policeNear(v.x, v.y, 60)
        if #police > 0 then
            return
        end
        local behavior = "tailgate"
        local tgtDriver = ctx:driverOf(tgt)
        local ____temp_6
        if tgtDriver ~= nil then
            ____temp_6 = tgtDriver.leader.id == v.id
        else
            ____temp_6 = self:isBehind(tgt)
        end
        local targetBehindMe = ____temp_6
        if targetBehindMe then
            behavior = "brake_check"
        elseif self.leader.id ~= tgt.id and self.p.aggression > 0.8 then
            behavior = "chase"
        end
        rage = {
            target = tgt.id,
            behavior = behavior,
            time = 0,
            lostTime = 0,
            brakeCheck = 0,
            cooldown = 0,
            rams = 0
        }
        self.rage = rage
        ctx.events:emit({type = "rage_start", vehicle = v.id, target = tgt.id, behavior = behavior})
        self:say(
            ctx,
            lineFor(self.rng, "furious", nil)
        )
        if behavior == "chase" then
            self:startChase(ctx)
        end
        return
    end
    rage.time = rage.time + dt
    rage.cooldown = rage.cooldown - dt
    if rage.brakeCheck > 0 then
        rage.brakeCheck = rage.brakeCheck - dt
    end
    local tgt = ctx:vehicle(rage.target)
    if tgt == nil or not tgt.alive then
        return self:endRage(ctx, "target gone")
    end
    local dist = math.sqrt((tgt.x - v.x) * (tgt.x - v.x) + (tgt.y - v.y) * (tgt.y - v.y))
    if dist > 120 then
        rage.lostTime = rage.lostTime + dt
    else
        rage.lostTime = 0
    end
    if rage.lostTime > 6 then
        return self:endRage(ctx, "lost target")
    end
    if emo.anger < 35 then
        return self:endRage(ctx, "calmed down")
    end
    if rage.time > 50 then
        return self:endRage(ctx, "gave up")
    end
    if #ctx:policeNear(v.x, v.y, 50) > 0 then
        emo.anger = math.min(emo.anger, 40)
        return self:endRage(ctx, "police nearby")
    end
    local tgtDriver = ctx:driverOf(tgt)
    local ____temp_7
    if tgtDriver ~= nil then
        ____temp_7 = tgtDriver.leader.id == v.id and tgtDriver.leader.gap < 18
    else
        ____temp_7 = self:isBehind(tgt) and dist < 18
    end
    local targetBehindMe = ____temp_7
    if rage.behavior == "tailgate" then
        if self.leader.id == rage.target and self.rng:chance(dt * 0.5) then
            self:flashHighBeams(ctx)
        end
        if self.leader.id == rage.target and self.rng:chance(dt * 0.35) then
            self:honk(ctx, true)
        end
        if rage.time > 8 and self.p.aggression > 0.82 and emo.anger > 88 and dist < 25 then
            rage.behavior = "ram"
            ctx.events:emit({type = "rage_start", vehicle = v.id, target = rage.target, behavior = "ram"})
            self:startChase(ctx)
        elseif targetBehindMe then
            rage.behavior = "brake_check"
        end
    elseif rage.behavior == "brake_check" then
        if targetBehindMe and rage.cooldown <= 0 and v.speed > 5 then
            rage.brakeCheck = self.rng:range(0.8, 1.4)
            rage.cooldown = self.rng:range(6, 10)
            ctx:offend(v, tgt, "brake_check", 1)
            ctx.events:emit({type = "rage_start", vehicle = v.id, target = rage.target, behavior = "brake_check"})
        end
        if not targetBehindMe and rage.cooldown <= 0 then
            rage.behavior = "tailgate"
        end
    end
end
function AIDriver.prototype.endRage(self, ctx, reason)
    self.rage = nil
    self.raging = false
    ctx.events:emit({type = "rage_end", vehicle = self.v.id, reason = reason})
    if self.state == "chasing" then
        self.state = "recovering"
    end
end
function AIDriver.prototype.isBehind(self, o)
    local v = self.v
    local dx = o.x - v.x
    local dy = o.y - v.y
    return dx * math.cos(v.heading) + dy * math.sin(v.heading) < -3 and math.abs(angleDiff(v.heading, o.heading)) < 0.8
end
function AIDriver.prototype.startChase(self, ctx)
    if self.v.mode ~= "physics" then
        self.v:enterPhysics()
    end
    self.state = "chasing"
    self.v.lights:setIndicator("off")
    local ____ = ctx
end
function AIDriver.prototype.onCrash(self, ctx, severity)
    if self.state == "chasing" and severity < 6 then
        return
    end
    self.state = "crashed"
    self.crashTimer = self.rng:range(2.5, 6) + severity * 0.3
    self.v.lights:setIndicator("off")
    local ____ = ctx
end
function AIDriver.prototype.physicsUpdate(self, dt, ctx)
    local v = self.v
    v.physicsTime = v.physicsTime + dt
    local lights = v.lights
    lights.brake = v.inputs.brake > 0.05 and 1 or 0
    lights.reverse = v.pt.gear < 0
    lights:update(dt, v.dyn.steerAngle)
    if self.followTarget >= 0 then
        self:followUpdate(dt, ctx)
        return
    end
    repeat
        local ____switch264 = self.state
        local ____cond264 = ____switch264 == "crashed"
        if ____cond264 then
            do
                brakeToStop(v, true)
                if v.physicsTime > 1 then
                    lights.hazard = true
                end
                if v.wrecked then
                    return
                end
                if v:absSpeed() < 0.5 then
                    self.crashTimer = self.crashTimer - dt
                end
                if self.crashTimer <= 0 then
                    if self.emo:canRage() and self.emo.target >= 0 and self.rage == nil then
                        self:updateRage(dt, ctx)
                        if self.rage ~= nil then
                            self.state = "chasing"
                            return
                        end
                    end
                    self.state = "recovering"
                    self.recoverLane = nil
                end
                return
            end
        end
        ____cond264 = ____cond264 or ____switch264 == "chasing"
        if ____cond264 then
            do
                local rage = self.rage
                if rage == nil then
                    self.state = "recovering"
                    return
                end
                self:updateRage(dt, ctx)
                local tgt = ctx:vehicle(rage.target)
                if tgt == nil or self.rage == nil then
                    self.state = "recovering"
                    return
                end
                local lead = rage.behavior == "ram" and 0.35 or 0.8
                local tx = tgt.x + tgt:vx() * lead
                local ty = tgt.y + tgt:vy() * lead
                local dist = math.sqrt((tx - v.x) * (tx - v.x) + (ty - v.y) * (ty - v.y))
                local tSpeed = tgt:absSpeed()
                local want = rage.behavior == "ram" and tSpeed + 6 or (dist > 20 and tSpeed + 8 or tSpeed)
                driveToward(
                    v,
                    tx,
                    ty,
                    clamp(want, 6, 40),
                    true
                )
                if self.rng:chance(dt * 0.6) then
                    self:honk(ctx, true)
                end
                if self.rng:chance(dt * 0.4) then
                    self:flashHighBeams(ctx)
                end
                lights.hazard = false
                return
            end
        end
        ____cond264 = ____cond264 or ____switch264 == "recovering"
        if ____cond264 then
            self:recoverUpdate(dt, ctx)
            return
        end
        do
            self.state = "recovering"
        end
    until true
end
function AIDriver.prototype.recoverUpdate(self, dt, ctx)
    local v = self.v
    local d = v.dyn
    v.lights.hazard = false
    if self.recoverLane == nil or self.wanderTimer <= 0 then
        self.wanderTimer = 2
        local hits = ctx.net:nearestLanes(d.x, d.y, 30, {})
        local best = nil
        local bestScore = math.huge
        for ____, h in ipairs(hits) do
            do
                local __continue279
                repeat
                    if h.s > h.lane.length - 15 then
                        __continue279 = true
                        break
                    end
                    local ang = math.abs(angleDiff(
                        d.yaw,
                        h.lane.path:headingAt(h.s)
                    ))
                    local score = h.dist + ang * 12
                    if score < bestScore then
                        bestScore = score
                        best = h
                    end
                    __continue279 = true
                until true
                if not __continue279 then
                    break
                end
            end
        end
        self.recoverLane = best ~= nil and best.lane or nil
        if self.recoverLane == nil then
            brakeToStop(v, false)
            v.despawnTimer = v.despawnTimer + dt * 20
            return
        end
    end
    self.wanderTimer = self.wanderTimer - dt
    local lane = self.recoverLane
    local pr = {s = 0, lateral = 0, d2 = 0}
    lane.path:project(d.x, d.y, pr)
    local target = {x = 0, y = 0, heading = 0}
    lane.path:sample(
        math.min(lane.length, pr.s + 9),
        target
    )
    driveToward(
        v,
        target.x,
        target.y,
        6,
        true
    )
    local heading = lane.path:headingAt(pr.s)
    local herr = math.abs(angleDiff(d.yaw, heading))
    if math.sqrt(pr.d2) < 0.9 and herr < 0.3 and d.u > 0.5 and pr.s < lane.length - 5 then
        v:enterLane(lane, pr.s, pr.lateral)
        v.latVel = 0
        self.latTarget = 0
        self.state = "drive"
        self:newDestination(ctx, lane)
        self.js:reset(lane.id)
    end
    if v.physicsTime > 90 then
        v.despawnTimer = v.despawnTimer + dt
    end
end
function AIDriver.prototype.followUpdate(self, dt, ctx)
    local v = self.v
    local tgt = ctx:vehicle(self.followTarget)
    if tgt == nil then
        self.followTarget = -1
        self.state = "recovering"
        return
    end
    local dx = tgt.x - v.x
    local dy = tgt.y - v.y
    local dist = math.sqrt(dx * dx + dy * dy)
    local tSpeed = tgt:absSpeed()
    local gapErr = dist - self.followGap
    local want = tSpeed + clamp(gapErr * 0.6, -tSpeed - 5, 25)
    if self.followGap < 2 then
        want = tSpeed + 10
    end
    if dist < self.followGap and tSpeed < 0.5 then
        brakeToStop(v, false)
        return
    end
    local back = self.followGap > 2 and 0 or -0.5
    local ax = tgt.x - math.cos(tgt.heading) * back + tgt:vx() * 0.4
    local ay = tgt.y - math.sin(tgt.heading) * back + tgt:vy() * 0.4
    driveToward(
        v,
        ax,
        ay,
        clamp(want, 0, 55),
        true
    )
    local ____ = dt
    local ____ = ctx
end
return ____exports
 end,
["src.core.law.violations"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local Map = ____lualib.Map
local __TS__New = ____lualib.__TS__New
local __TS__ArraySplice = ____lualib.__TS__ArraySplice
local __TS__ArrayReduce = ____lualib.__TS__ArrayReduce
local ____exports = {}
____exports.VIOLATIONS = {
    red_light = {
        label = "Ran a red light",
        fine = 400,
        points = 3,
        heat = 1,
        cooldown = 4
    },
    right_on_red_no_stop = {
        label = "Right on red without stopping",
        fine = 150,
        points = 1,
        heat = 1,
        cooldown = 4
    },
    stop_sign = {
        label = "Failed to stop at stop sign",
        fine = 200,
        points = 2,
        heat = 1,
        cooldown = 4
    },
    speeding_minor = {
        label = "Speeding (10–25% over)",
        fine = 120,
        points = 1,
        heat = 1,
        cooldown = 25
    },
    speeding_major = {
        label = "Speeding (25–50% over)",
        fine = 350,
        points = 3,
        heat = 1,
        cooldown = 25
    },
    speeding_extreme = {
        label = "Excessive speed (50%+ over)",
        fine = 900,
        points = 6,
        heat = 2,
        cooldown = 25
    },
    wrong_way = {
        label = "Driving the wrong way",
        fine = 500,
        points = 4,
        heat = 2,
        cooldown = 20
    },
    sidewalk = {
        label = "Driving on the sidewalk",
        fine = 250,
        points = 2,
        heat = 1,
        cooldown = 15
    },
    collision = {
        label = "Careless driving (collision)",
        fine = 300,
        points = 3,
        heat = 1,
        cooldown = 3
    },
    hit_and_run = {
        label = "Leaving the scene of a collision",
        fine = 1500,
        points = 6,
        heat = 3,
        cooldown = 30
    },
    no_signal_lane = {
        label = "Lane change without signalling",
        fine = 60,
        points = 0,
        heat = 1,
        cooldown = 15
    },
    no_signal_turn = {
        label = "Turn without signalling",
        fine = 80,
        points = 1,
        heat = 1,
        cooldown = 10
    },
    illegal_turn = {
        label = "Illegal turn",
        fine = 200,
        points = 2,
        heat = 1,
        cooldown = 8
    },
    no_headlights = {
        label = "No headlights after dark",
        fine = 90,
        points = 1,
        heat = 1,
        cooldown = 60
    },
    seatbelt = {
        label = "Not wearing a seatbelt",
        fine = 70,
        points = 0,
        heat = 1,
        cooldown = 60
    },
    fail_yield_emergency = {
        label = "Failure to yield to emergency vehicle",
        fine = 450,
        points = 3,
        heat = 2,
        cooldown = 30
    },
    tailgating = {
        label = "Following too closely",
        fine = 120,
        points = 2,
        heat = 1,
        cooldown = 20
    },
    reckless = {
        label = "Reckless driving",
        fine = 800,
        points = 5,
        heat = 2,
        cooldown = 20
    },
    blocking_box = {
        label = "Blocking the intersection",
        fine = 150,
        points = 1,
        heat = 1,
        cooldown = 20
    },
    horn_abuse = {
        label = "Unnecessary use of horn",
        fine = 50,
        points = 0,
        heat = 1,
        cooldown = 30
    },
    evading = {
        label = "Evading police",
        fine = 2500,
        points = 8,
        heat = 3,
        cooldown = 60
    }
}
--- The player's driving record and wallet.
____exports.DrivingRecord = __TS__Class()
local DrivingRecord = ____exports.DrivingRecord
DrivingRecord.name = "DrivingRecord"
function DrivingRecord.prototype.____constructor(self)
    self.money = 5000
    self.points = 0
    self.maxPoints = 12
    self.history = {}
    self.pending = {}
    self.heat = 0
    self.cleanDistance = 0
    self.ticketsIssued = 0
    self.totalFines = 0
    self.lastCharged = __TS__New(Map)
end
function DrivingRecord.prototype.suspended(self)
    return self.points >= self.maxPoints
end
function DrivingRecord.prototype.canCharge(self, kind, now)
    local last = self.lastCharged:get(kind)
    return last == nil or now - last >= ____exports.VIOLATIONS[kind].cooldown
end
function DrivingRecord.prototype.add(self, kind, now, witness, x, y)
    local d = ____exports.VIOLATIONS[kind]
    self.lastCharged:set(kind, now)
    local rec = {
        kind = kind,
        label = d.label,
        time = now,
        fine = d.fine,
        points = d.points,
        witness = witness,
        paid = false,
        x = x,
        y = y
    }
    local ____self_history_0 = self.history
    ____self_history_0[#____self_history_0 + 1] = rec
    if #self.history > 200 then
        table.remove(self.history, 1)
    end
    self.cleanDistance = 0
    if witness == "camera" then
        self:charge({rec})
    elseif witness == "police" then
        local ____self_pending_1 = self.pending
        ____self_pending_1[#____self_pending_1 + 1] = rec
    end
    return rec
end
function DrivingRecord.prototype.charge(self, recs)
    local total = 0
    for ____, r in ipairs(recs) do
        do
            local __continue10
            repeat
                if r.paid then
                    __continue10 = true
                    break
                end
                r.paid = true
                total = total + r.fine
                self.points = self.points + r.points
                __continue10 = true
            until true
            if not __continue10 then
                break
            end
        end
    end
    self.money = self.money - total
    self.totalFines = self.totalFines + total
    return total
end
function DrivingRecord.prototype.issueTicket(self)
    local recs = __TS__ArraySplice(self.pending, 0, #self.pending)
    local pts = __TS__ArrayReduce(
        recs,
        function(____, a, r) return a + r.points end,
        0
    )
    local total = self:charge(recs)
    self.ticketsIssued = self.ticketsIssued + 1
    return {total = total, points = pts, count = #recs}
end
function DrivingRecord.prototype.driveClean(self, meters)
    self.cleanDistance = self.cleanDistance + meters
    if self.cleanDistance > 2000 and self.points > 0 then
        self.points = self.points - 1
        self.cleanDistance = 0
    end
end
return ____exports
 end,
["src.core.law.playerMonitor"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local Map = ____lualib.Map
local __TS__New = ____lualib.__TS__New
local ____exports = {}
local ____scalar = require("src.core.math.scalar")
local angleDiff = ____scalar.angleDiff
local DEG = ____scalar.DEG
--- Watches the player's vehicle against the road network: which lane it is
-- in, whether it is going the wrong way, what it did at each junction (red
-- lights, stop signs, signalling, illegal turns), speeding and more. Emits
-- raw observations; the world decides who witnessed them.
____exports.PlayerMonitor = __TS__Class()
local PlayerMonitor = ____exports.PlayerMonitor
PlayerMonitor.name = "PlayerMonitor"
function PlayerMonitor.prototype.____constructor(self)
    self.info = {
        roadName = "",
        speedLimit = 13.9,
        onRoad = true,
        wrongWay = false,
        inJunction = false,
        lane = nil,
        ahead = nil,
        leaderGap = math.huge,
        headway = math.huge
    }
    self.hits = {}
    self.lastLane = nil
    self.wrongWayTime = 0
    self.offRoadTime = 0
    self.speedingTime = 0
    self.darkNoLightsTime = 0
    self.unbeltedTime = 0
    self.tailgateTime = 0
    self.skidTime = 0
    self.emergencyBehindTime = 0
    self.hornHeld = 0
    self.boxStopTime = 0
    self.entryLane = nil
    self.entrySnapshot = __TS__New(Map)
    self.entryNode = nil
    self.minSpeedNearLine = math.huge
    self.stoppedNearLine = false
    self.signalLeftAt = -100
    self.signalRightAt = -100
    self.hazardAt = -100
    self.hitRun = nil
    self.seatbelt = true
end
function PlayerMonitor.prototype.update(self, dt, p, ctx, out)
    local now = ctx.time
    local info = self.info
    local speed = p:absSpeed()
    local lights = p.lights
    if lights.indicator == "left" or lights.hazard then
        self.signalLeftAt = now
    end
    if lights.indicator == "right" or lights.hazard then
        self.signalRightAt = now
    end
    if lights.hazard then
        self.hazardAt = now
    end
    self.hits = {}
    ctx.net:nearestLanes(p.x, p.y, 7, self.hits)
    local aligned = nil
    local opposite = nil
    for ____, h in ipairs(self.hits) do
        do
            local __continue7
            repeat
                local half = h.lane.width / 2 + 0.4
                if math.abs(h.lateral) > half or h.s <= 0.01 or h.s >= h.lane.length - 0.01 then
                    __continue7 = true
                    break
                end
                local d = math.abs(angleDiff(
                    p.heading,
                    h.lane.path:headingAt(h.s)
                ))
                if d < 70 * DEG and aligned == nil then
                    aligned = h
                end
                if d > 110 * DEG and opposite == nil then
                    opposite = h
                end
                __continue7 = true
            until true
            if not __continue7 then
                break
            end
        end
    end
    local node = ctx.net:nodeAt(p.x, p.y)
    info.inJunction = node ~= nil
    local lane = aligned ~= nil and aligned.lane or (opposite ~= nil and opposite.lane or nil)
    info.lane = aligned ~= nil and aligned.lane or nil
    local nearRoad = #self.hits > 0 and self.hits[1].dist < self.hits[1].lane.width / 2 + 1.3
    info.onRoad = lane ~= nil or node ~= nil or nearRoad
    if lane ~= nil then
        info.roadName = lane.road.name
        info.speedLimit = lane.speedLimit
    end
    local wrong = aligned == nil and opposite ~= nil and p.speed > 2 and node == nil
    info.wrongWay = wrong
    if wrong then
        self.wrongWayTime = self.wrongWayTime + dt
        if self.wrongWayTime > 1.5 then
            out[#out + 1] = {kind = "wrong_way"}
        end
    else
        self.wrongWayTime = 0
    end
    if not info.onRoad and speed > 4 then
        self.offRoadTime = self.offRoadTime + dt
        if self.offRoadTime > 1 then
            out[#out + 1] = {kind = "sidewalk"}
        end
    else
        self.offRoadTime = 0
    end
    local cur = aligned ~= nil and aligned.lane or nil
    if cur ~= nil and self.lastLane ~= nil and cur ~= self.lastLane and cur.road == self.lastLane.road and cur.forward == self.lastLane.forward and speed > 3 then
        local toLeft = cur.index < self.lastLane.index
        local last = toLeft and self.signalLeftAt or self.signalRightAt
        if now - last > 3 then
            out[#out + 1] = {kind = "no_signal_lane"}
        end
    end
    if cur ~= nil then
        local toLine = cur.length - aligned.s
        if toLine < 18 then
            self.minSpeedNearLine = math.min(self.minSpeedNearLine, speed)
            if speed < 0.4 and toLine < 7 then
                self.stoppedNearLine = true
            end
        end
        if cur ~= self.lastLane and cur.road ~= (self.lastLane ~= nil and self.lastLane.road or nil) then
            if self.entryLane ~= nil and self.entryNode ~= nil and cur.from == self.entryNode then
                self:evaluateMovement(ctx, cur, out)
            end
            self.entryLane = nil
            self.entryNode = nil
            self.minSpeedNearLine = math.huge
            self.stoppedNearLine = false
        end
        self.lastLane = cur
    end
    if node ~= nil and self.entryLane == nil and self.lastLane ~= nil and self.lastLane.to == node then
        self.entryLane = self.lastLane
        self.entryNode = node
        self.entrySnapshot = __TS__New(Map)
        if node.control == "signal" then
            local sig = ctx.signals[node.signalId + 1]
            for ____, ap in ipairs(node.approaches) do
                for ____, l in ipairs(ap.lanes) do
                    for ____, c in ipairs(l.outgoing) do
                        self.entrySnapshot:set(
                            c.id,
                            sig:indication(c)
                        )
                    end
                end
            end
        end
    end
    if node ~= nil and speed < 0.5 and self.entryLane ~= nil and node.control == "signal" then
        local sig = ctx.signals[node.signalId + 1]
        local anyGreen = false
        for ____, c in ipairs(self.entryLane.outgoing) do
            if sig:indication(c) == "green" or sig:indication(c) == "permissive" then
                anyGreen = true
            end
        end
        if not anyGreen then
            self.boxStopTime = self.boxStopTime + dt
            if self.boxStopTime > 4 then
                out[#out + 1] = {kind = "blocking_box"}
            end
        else
            self.boxStopTime = 0
        end
    else
        self.boxStopTime = 0
    end
    if lane ~= nil and node == nil then
        local lim = lane.speedLimit
        local over = (speed - lim) / lim
        if over > 0.1 and speed - lim > 1 then
            self.speedingTime = self.speedingTime + dt
            if self.speedingTime > 2 then
                local kind = over > 0.5 and "speeding_extreme" or (over > 0.25 and "speeding_major" or "speeding_minor")
                out[#out + 1] = {
                    kind = kind,
                    speedCamera = lane.road.speedCamera and self:nearSpeedCamera(lane, aligned ~= nil and aligned.s or 0)
                }
            end
        else
            self.speedingTime = 0
        end
    end
    if ctx.env.dark and not lights:anyHeadlight() and speed > 3 then
        self.darkNoLightsTime = self.darkNoLightsTime + dt
        if self.darkNoLightsTime > 10 then
            out[#out + 1] = {kind = "no_headlights"}
        end
    else
        self.darkNoLightsTime = 0
    end
    if not self.seatbelt and speed > 5.5 then
        self.unbeltedTime = self.unbeltedTime + dt
        if self.unbeltedTime > 5 then
            out[#out + 1] = {kind = "seatbelt"}
        end
    else
        self.unbeltedTime = 0
    end
    if lights.horn then
        self.hornHeld = self.hornHeld + dt
        if self.hornHeld > 4 then
            out[#out + 1] = {kind = "horn_abuse"}
        end
    else
        self.hornHeld = math.max(0, self.hornHeld - dt * 0.5)
    end
    info.leaderGap = math.huge
    info.headway = math.huge
    if cur ~= nil and aligned ~= nil then
        local gap = math.huge
        do
            local i = 0
            while i < cur.occCount do
                do
                    local __continue56
                    repeat
                        local e = cur.occ[i + 1]
                        if e.occ.id == p.id then
                            __continue56 = true
                            break
                        end
                        local d = e.s - aligned.s - (e.occ.length + p.length) / 2
                        if d > -0.5 and d < gap and math.abs(e.lat - aligned.lateral) < 2.2 then
                            gap = d
                        end
                        __continue56 = true
                    until true
                    if not __continue56 then
                        break
                    end
                end
                i = i + 1
            end
        end
        info.leaderGap = gap
        info.headway = speed > 1 and gap / speed or math.huge
        if speed > 12 and info.headway < 0.6 then
            self.tailgateTime = self.tailgateTime + dt
            if self.tailgateTime > 4 then
                out[#out + 1] = {kind = "tailgating"}
            end
        else
            self.tailgateTime = 0
        end
    end
    if p.dyn.skid > 0.6 and speed > 6 then
        self.skidTime = self.skidTime + dt
        if self.skidTime > 2 then
            out[#out + 1] = {kind = "reckless"}
        end
    else
        self.skidTime = math.max(0, self.skidTime - dt)
    end
    local emergencyBehind = false
    for ____, e in ipairs(ctx.vehicles) do
        do
            local __continue65
            repeat
                if not e:sirenOn() or e == p then
                    __continue65 = true
                    break
                end
                local dx = e.x - p.x
                local dy = e.y - p.y
                if dx * dx + dy * dy > 45 * 45 then
                    __continue65 = true
                    break
                end
                local behind = dx * math.cos(p.heading) + dy * math.sin(p.heading) < -3
                if behind and math.abs(angleDiff(p.heading, e.heading)) < 45 * DEG and e:absSpeed() > 4 then
                    emergencyBehind = true
                end
                __continue65 = true
            until true
            if not __continue65 then
                break
            end
        end
    end
    if emergencyBehind and speed > 4 then
        self.emergencyBehindTime = self.emergencyBehindTime + dt
        if self.emergencyBehindTime > 7 then
            out[#out + 1] = {kind = "fail_yield_emergency"}
        end
    else
        self.emergencyBehindTime = math.max(0, self.emergencyBehindTime - dt * 2)
    end
    local hr = self.hitRun
    if hr ~= nil then
        if speed < 0.5 then
            hr.stoppedFor = hr.stoppedFor + dt
        end
        local dx = p.x - hr.x
        local dy = p.y - hr.y
        if hr.stoppedFor > 3 or now - hr.time > 40 then
            self.hitRun = nil
        elseif dx * dx + dy * dy > 90 * 90 then
            out[#out + 1] = {kind = "hit_and_run"}
            self.hitRun = nil
        end
    end
    self:updateAhead(ctx, p, cur, aligned)
end
function PlayerMonitor.prototype.hazardThanks(self, now)
    return now - self.hazardAt < 1.5
end
function PlayerMonitor.prototype.nearSpeedCamera(self, lane, s)
    local mid = lane.length / 2
    return math.abs(s - mid) < 30
end
function PlayerMonitor.prototype.evaluateMovement(self, ctx, exitLane, out)
    local entry = self.entryLane
    local node = self.entryNode
    local c = entry:connectorTo(exitLane.road)
    if c == nil then
        local any = nil
        local ap = entry.approach
        if ap ~= nil then
            for ____, l in ipairs(ap.lanes) do
                local k = l:connectorTo(exitLane.road)
                if k ~= nil then
                    any = k
                end
            end
        end
        if any == nil then
            out[#out + 1] = {kind = "illegal_turn"}
            return
        end
        c = any
    end
    local now = ctx.time
    if c.turn == "left" and now - self.signalLeftAt > 6 then
        out[#out + 1] = {kind = "no_signal_turn"}
    end
    if c.turn == "right" and now - self.signalRightAt > 6 then
        out[#out + 1] = {kind = "no_signal_turn"}
    end
    local ap = entry.approach
    if node.control == "signal" then
        local ind = self.entrySnapshot:get(c.id)
        if ind == "red" then
            local sig = ctx.signals[node.signalId + 1]
            if c.turn == "right" and sig.allowRightOnRed then
                if not self.stoppedNearLine then
                    out[#out + 1] = {kind = "right_on_red_no_stop"}
                end
            else
                out[#out + 1] = {kind = "red_light", cameraNode = node.redLightCamera and node or nil}
            end
        elseif (ind == "flash_red" or ind == "dark") and self.minSpeedNearLine > 1.5 then
            out[#out + 1] = {kind = "stop_sign"}
        end
    elseif ap ~= nil and ap.sign == "stop" and self.minSpeedNearLine > 1.4 then
        out[#out + 1] = {kind = "stop_sign"}
    end
end
function PlayerMonitor.prototype.updateAhead(self, ctx, p, cur, hit)
    local info = self.info
    if cur == nil or hit == nil then
        if not info.inJunction then
            info.ahead = nil
        end
        return
    end
    local dist = cur.length - hit.s
    local ap = cur.approach
    if ap == nil or dist > 160 then
        info.ahead = nil
        return
    end
    local node = ap.node
    local control = "none"
    local head = nil
    local indication = nil
    if node.control == "signal" then
        control = "signal"
        local sig = ctx.signals[node.signalId + 1]
        head = sig:head(ap)
        local want = p.lights.indicator == "left" and "left" or (p.lights.indicator == "right" and "right" or "straight")
        local c = nil
        for ____, k in ipairs(cur.outgoing) do
            if k.turn == want then
                c = k
            end
        end
        if c == nil and #cur.outgoing > 0 then
            c = cur.outgoing[1]
        end
        if c ~= nil then
            indication = sig:indication(c)
        end
    elseif ap.sign == "stop" then
        control = "stop"
    elseif ap.sign == "yield" then
        control = "yield"
    end
    info.ahead = {
        distance = dist,
        control = control,
        head = head,
        indication = indication,
        nodeName = node.name,
        camera = node.redLightCamera
    }
end
return ____exports
 end,
["src.core.law.police"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local __TS__New = ____lualib.__TS__New
local __TS__ArrayIndexOf = ____lualib.__TS__ArrayIndexOf
local __TS__ArraySplice = ____lualib.__TS__ArraySplice
local ____exports = {}
local ____rng = require("src.core.math.rng")
local Rng = ____rng.Rng
local ____scalar = require("src.core.math.scalar")
local hypot = ____scalar.hypot
local ____speech = require("src.core.ai.speech")
local POLICE_LINES = ____speech.POLICE_LINES
--- Traffic police: dispatches the nearest patrol car when an officer
-- witnesses a violation, performs the pull-over (follow, wait for the player
-- to stop, write the ticket) and escalates to a pursuit with more units when
-- the player flees. Losing all units for long enough ends the pursuit.
____exports.PoliceSystem = __TS__Class()
local PoliceSystem = ____exports.PoliceSystem
PoliceSystem.name = "PoliceSystem"
function PoliceSystem.prototype.____constructor(self)
    self.stage = "none"
    self.units = {}
    self.timer = 0
    self.stillTime = 0
    self.fleeTime = 0
    self.lostTime = 0
    self.routeTimer = 0
    self.pursuitTime = 0
    self.ticketCountdown = 0
    self.rng = __TS__New(Rng, 911)
    self.spawnUnit = nil
end
function PoliceSystem.prototype.active(self)
    return self.stage ~= "none"
end
function PoliceSystem.prototype.report(self, ctx, unit, rec, heat)
    local p = ctx.player
    if p == nil then
        return
    end
    rec.heat = math.min(
        5,
        math.max(rec.heat, self.stage == "pursuit" and rec.heat + heat * 0.5 or heat)
    )
    ctx.events:emit({
        type = "heat",
        level = math.floor(rec.heat),
        reason = "violation witnessed"
    })
    if self.stage == "none" then
        local u = unit or self:nearestFreeUnit(ctx, p.x, p.y)
        if u == nil then
            return
        end
        self:assign(ctx, u)
        self.stage = "responding"
        self.timer = 0
        ctx.events:emit({
            type = "dispatch",
            unit = u.id,
            kind = "police",
            x = p.x,
            y = p.y,
            reason = "traffic violation"
        })
    elseif self.stage == "pursuit" and rec.heat >= 3 and #self.units < math.floor(rec.heat) then
        self:addUnit(ctx)
    end
end
function PoliceSystem.prototype.nearestFreeUnit(self, ctx, x, y)
    local best = nil
    local bd = math.huge
    for ____, v in ipairs(ctx.vehicles) do
        do
            local __continue10
            repeat
                if v.role ~= "police" or not v.alive or v.wrecked or __TS__ArrayIndexOf(self.units, v) >= 0 then
                    __continue10 = true
                    break
                end
                local d = (v.x - x) * (v.x - x) + (v.y - y) * (v.y - y)
                if d < bd then
                    bd = d
                    best = v
                end
                __continue10 = true
            until true
            if not __continue10 then
                break
            end
        end
    end
    return best
end
function PoliceSystem.prototype.assign(self, ctx, u)
    local ____self_units_0 = self.units
    ____self_units_0[#____self_units_0 + 1] = u
    local d = ctx:driverOf(u)
    if d ~= nil then
        d.emergency = true
        d.rage = nil
    end
    u.lights.siren = "wail"
    u.lights.lightbar = true
end
function PoliceSystem.prototype.addUnit(self, ctx)
    local p = ctx.player
    if p == nil then
        return
    end
    local u = self:nearestFreeUnit(ctx, p.x, p.y)
    if (u == nil or hypot(u.x - p.x, u.y - p.y) > 400) and self.spawnUnit ~= nil then
        u = self.spawnUnit(p.x, p.y)
    end
    if u == nil then
        return
    end
    self:assign(ctx, u)
    ctx.events:emit({
        type = "dispatch",
        unit = u.id,
        kind = "police",
        x = p.x,
        y = p.y,
        reason = "backup"
    })
end
function PoliceSystem.prototype.release(self, ctx)
    for ____, u in ipairs(self.units) do
        local d = ctx:driverOf(u)
        if d ~= nil then
            d.emergency = false
            d.followTarget = -1
            d.headFor = nil
            if u.mode == "physics" then
                d.state = "recovering"
            end
        end
        u.lights.siren = "off"
        u.lights.lightbar = false
    end
    __TS__ArraySplice(self.units, 0, #self.units)
    self.stage = "none"
    self.stillTime = 0
    self.fleeTime = 0
    self.lostTime = 0
    self.pursuitTime = 0
    self.ticketCountdown = 0
end
function PoliceSystem.prototype.seen(self, ctx, p, range)
    for ____, u in ipairs(self.units) do
        local dx = u.x - p.x
        local dy = u.y - p.y
        if dx * dx + dy * dy < range * range and ctx:los(u.x, u.y, p.x, p.y) then
            return u
        end
    end
    return nil
end
function PoliceSystem.prototype.update(self, dt, ctx, rec)
    local p = ctx.player
    if p == nil or self.stage == "none" then
        return
    end
    do
        local i = #self.units - 1
        while i >= 0 do
            local u = self.units[i + 1]
            if not u.alive or u.wrecked then
                __TS__ArraySplice(self.units, i, 1)
            end
            i = i - 1
        end
    end
    if #self.units == 0 then
        if self.stage == "pursuit" or self.stage == "searching" then
            self:addUnit(ctx)
            if #self.units == 0 then
                ctx.events:emit({type = "pullover", police = -1, stage = "escaped"})
                rec.heat = 0
                self:release(ctx)
            end
        else
            self:release(ctx)
        end
        return
    end
    self.timer = self.timer + dt
    local lead = self.units[1]
    local leadDriver = ctx:driverOf(lead)
    local dist = hypot(lead.x - p.x, lead.y - p.y)
    local speed = p:absSpeed()
    self.routeTimer = self.routeTimer - dt
    for ____, u in ipairs(self.units) do
        do
            local __continue37
            repeat
                local d = ctx:driverOf(u)
                if d == nil then
                    __continue37 = true
                    break
                end
                local du = hypot(u.x - p.x, u.y - p.y)
                local close = du < 55 and ctx:los(u.x, u.y, p.x, p.y)
                if close and d.followTarget < 0 then
                    d.followTarget = p.id
                    d.followGap = self.stage == "pursuit" and (rec.heat >= 3 and self.rng:chance(0.5) and 1 or 5) or 11
                    u:enterPhysics()
                elseif not close and d.followTarget >= 0 and du > 140 then
                    d.followTarget = -1
                    d.state = "recovering"
                end
                if d.followTarget < 0 and u.mode == "lane" and self.routeTimer <= 0 then
                    local hits = ctx.net:nearestLanes(p.x, p.y, 25, {})
                    if #hits > 0 then
                        d:routeTo(ctx, hits[1].lane.road, hits[1].lane.forward)
                    end
                end
                __continue37 = true
            until true
            if not __continue37 then
                break
            end
        end
    end
    if self.routeTimer <= 0 then
        self.routeTimer = 3
    end
    repeat
        local ____switch45 = self.stage
        local ____cond45 = ____switch45 == "responding"
        if ____cond45 then
            if leadDriver ~= nil and leadDriver.followTarget == p.id then
                self.stage = "pullover"
                self.timer = 0
                ctx.events:emit({type = "pullover", police = lead.id, stage = "request"})
                ctx.events:emit({
                    type = "speech",
                    vehicle = lead.id,
                    text = self.rng:pick(POLICE_LINES),
                    mood = "calm"
                })
            elseif self.timer > 90 then
                rec.heat = 0
                self:release(ctx)
            end
            break
        end
        ____cond45 = ____cond45 or ____switch45 == "pullover"
        if ____cond45 then
            do
                if speed < 0.6 and dist < 35 then
                    self.stillTime = self.stillTime + dt
                else
                    self.stillTime = 0
                end
                if speed > 13 or dist > 100 then
                    self.fleeTime = self.fleeTime + dt
                else
                    self.fleeTime = math.max(0, self.fleeTime - dt)
                end
                if self.stillTime > 2.5 then
                    self.stage = "stopped"
                    self.timer = 0
                    self.ticketCountdown = 6
                    ctx.events:emit({type = "pullover", police = lead.id, stage = "stopped"})
                elseif self.fleeTime > 6 then
                    self.stage = "pursuit"
                    self.pursuitTime = 0
                    ctx.events:emit({type = "pullover", police = lead.id, stage = "fled"})
                    rec.heat = math.max(rec.heat, 2)
                    ctx.events:emit({
                        type = "heat",
                        level = math.floor(rec.heat),
                        reason = "evading"
                    })
                    self:onFlee(ctx, rec)
                elseif self.timer > 8 and self.rng:chance(dt * 0.3) then
                    ctx.events:emit({
                        type = "speech",
                        vehicle = lead.id,
                        text = self.rng:pick(POLICE_LINES),
                        mood = "calm"
                    })
                end
                break
            end
        end
        ____cond45 = ____cond45 or ____switch45 == "stopped"
        if ____cond45 then
            do
                self.ticketCountdown = self.ticketCountdown - dt
                if speed > 3 then
                    self.stage = "pursuit"
                    self.pursuitTime = 0
                    rec.heat = math.max(rec.heat, 2)
                    ctx.events:emit({type = "pullover", police = lead.id, stage = "fled"})
                    self:onFlee(ctx, rec)
                elseif self.ticketCountdown <= 0 then
                    local t = rec:issueTicket()
                    ctx.events:emit({
                        type = "ticket",
                        vehicle = p.id,
                        total = t.total,
                        points = t.points,
                        reason = t.count == 1 and "1 violation" or tostring(t.count) .. " violations"
                    })
                    ctx.events:emit({type = "pullover", police = lead.id, stage = "issued"})
                    rec.heat = 0
                    ctx.events:emit({type = "heat", level = 0, reason = "ticket issued"})
                    self:release(ctx)
                end
                break
            end
        end
        ____cond45 = ____cond45 or (____switch45 == "pursuit" or ____switch45 == "searching")
        if ____cond45 then
            do
                self.pursuitTime = self.pursuitTime + dt
                local seer = self:seen(ctx, p, 150)
                if seer ~= nil then
                    self.lostTime = 0
                    if self.stage == "searching" then
                        self.stage = "pursuit"
                    end
                else
                    self.lostTime = self.lostTime + dt
                    if self.lostTime > 10 then
                        self.stage = "searching"
                    end
                end
                local want = math.min(
                    5,
                    2 + math.floor(self.pursuitTime / 45)
                )
                if want > rec.heat then
                    rec.heat = want
                    ctx.events:emit({type = "heat", level = want, reason = "pursuit continues"})
                end
                if #self.units < math.min(
                    4,
                    math.floor(rec.heat)
                ) and self.rng:chance(dt * 0.2) then
                    self:addUnit(ctx)
                end
                if speed < 0.5 and dist < 16 then
                    self.stillTime = self.stillTime + dt
                else
                    self.stillTime = 0
                end
                if self.stillTime > 4 then
                    local t = rec:issueTicket()
                    rec.money = rec.money - 1000
                    ctx.events:emit({
                        type = "ticket",
                        vehicle = p.id,
                        total = t.total + 1000,
                        points = t.points,
                        reason = "arrested: fines + impound fee"
                    })
                    ctx.events:emit({type = "pullover", police = lead.id, stage = "issued"})
                    rec.heat = 0
                    ctx.events:emit({type = "heat", level = 0, reason = "arrested"})
                    self:release(ctx)
                elseif self.lostTime > 22 then
                    ctx.events:emit({type = "pullover", police = lead.id, stage = "escaped"})
                    rec.heat = 0
                    ctx.events:emit({type = "heat", level = 0, reason = "escaped"})
                    self:release(ctx)
                end
                break
            end
        end
        do
            break
        end
    until true
    local ____ = leadDriver
end
function PoliceSystem.prototype.onFlee(self, ctx, rec)
    local p = ctx.player
    if p ~= nil then
        local r = rec:add(
            "evading",
            ctx.time,
            "police",
            p.x,
            p.y
        )
        ctx.events:emit({
            type = "violation",
            vehicle = p.id,
            kind = r.kind,
            label = r.label,
            fine = r.fine,
            points = r.points,
            witnessed = "police",
            x = p.x,
            y = p.y
        })
    end
    for ____, u in ipairs(self.units) do
        local d = ctx:driverOf(u)
        if d ~= nil and d.followTarget >= 0 then
            d.followGap = 5
        end
    end
    self:addUnit(ctx)
end
return ____exports
 end,
["src.core.physics.collision"] = function(...) 
--[[ Generated with https://github.com/TypeScriptToLua/TypeScriptToLua ]]
local ____exports = {}
local function support(b, dx, dy, out)
    local c = math.cos(b.heading)
    local s = math.sin(b.heading)
    local fx = dx * c + dy * s
    local fy = -dx * s + dy * c
    local lx = fx >= 0 and b.hl or -b.hl
    local ly = fy >= 0 and b.hw or -b.hw
    out.x = b.x + lx * c - ly * s
    out.y = b.y + lx * s + ly * c
end
local sa = {x = 0, y = 0}
local sb = {x = 0, y = 0}
--- Separating-axis test for two oriented boxes. Returns true and fills `out` on overlap.
function ____exports.boxContact(a, b, out)
    local ca = math.cos(a.heading)
    local sA = math.sin(a.heading)
    local cb = math.cos(b.heading)
    local sB = math.sin(b.heading)
    local dx = b.x - a.x
    local dy = b.y - a.y
    local best = math.huge
    local bnx = 0
    local bny = 0
    do
        local i = 0
        while i < 4 do
            local nx
            local ny
            if i == 0 then
                nx = ca
                ny = sA
            elseif i == 1 then
                nx = -sA
                ny = ca
            elseif i == 2 then
                nx = cb
                ny = sB
            else
                nx = -sB
                ny = cb
            end
            local rA = a.hl * math.abs(ca * nx + sA * ny) + a.hw * math.abs(-sA * nx + ca * ny)
            local rB = b.hl * math.abs(cb * nx + sB * ny) + b.hw * math.abs(-sB * nx + cb * ny)
            local d = dx * nx + dy * ny
            local overlap = rA + rB - math.abs(d)
            if overlap <= 0 then
                return false
            end
            if overlap < best then
                best = overlap
                if d < 0 then
                    bnx = -nx
                    bny = -ny
                else
                    bnx = nx
                    bny = ny
                end
            end
            i = i + 1
        end
    end
    support(a, bnx, bny, sa)
    support(b, -bnx, -bny, sb)
    out.nx = bnx
    out.ny = bny
    out.depth = best
    out.px = (sa.x + sb.x) / 2
    out.py = (sa.y + sb.y) / 2
    return true
end
--- Impulse response between two rigid bodies. Returns the normal impulse (N·s).
-- Also separates the bodies along the normal.
function ____exports.resolveBodies(a, b, c, restitution, friction)
    local rax = c.px - a.x
    local ray = c.py - a.y
    local rbx = c.px - b.x
    local rby = c.py - b.y
    local vax = a:vx() - a.r * ray
    local vay = a:vy() + a.r * rax
    local vbx = b:vx() - b.r * rby
    local vby = b:vy() + b.r * rbx
    local rvx = vbx - vax
    local rvy = vby - vay
    local vn = rvx * c.nx + rvy * c.ny
    local invA = 1 / a.mass
    local invB = 1 / b.mass
    local corr = math.max(0, c.depth - 0.01) * 0.8 / (invA + invB)
    a.x = a.x - c.nx * corr * invA
    a.y = a.y - c.ny * corr * invA
    b.x = b.x + c.nx * corr * invB
    b.y = b.y + c.ny * corr * invB
    if vn >= 0 then
        return 0
    end
    local raN = rax * c.ny - ray * c.nx
    local rbN = rbx * c.ny - rby * c.nx
    local denom = invA + invB + raN * raN / a.iz + rbN * rbN / b.iz
    local j = -(1 + restitution) * vn / denom
    a:applyImpulse(-c.nx * j, -c.ny * j, c.px, c.py)
    b:applyImpulse(c.nx * j, c.ny * j, c.px, c.py)
    local tx = -c.ny
    local ty = c.nx
    local vt = rvx * tx + rvy * ty
    local raT = rax * ty - ray * tx
    local rbT = rbx * ty - rby * tx
    local denomT = invA + invB + raT * raT / a.iz + rbT * rbT / b.iz
    local jt = -vt / denomT
    local maxF = friction * j
    if jt > maxF then
        jt = maxF
    end
    if jt < -maxF then
        jt = -maxF
    end
    a:applyImpulse(-tx * jt, -ty * jt, c.px, c.py)
    b:applyImpulse(tx * jt, ty * jt, c.px, c.py)
    return j
end
--- Impulse response against an immovable obstacle (normal points from body into obstacle).
function ____exports.resolveStatic(a, c, restitution, friction)
    local rax = c.px - a.x
    local ray = c.py - a.y
    a.x = a.x - c.nx * math.max(0, c.depth - 0.005)
    a.y = a.y - c.ny * math.max(0, c.depth - 0.005)
    local vax = a:vx() - a.r * ray
    local vay = a:vy() + a.r * rax
    local vn = -(vax * c.nx + vay * c.ny)
    if vn >= 0 then
        return 0
    end
    local invA = 1 / a.mass
    local raN = rax * c.ny - ray * c.nx
    local j = -(1 + restitution) * vn / (invA + raN * raN / a.iz)
    a:applyImpulse(-c.nx * j, -c.ny * j, c.px, c.py)
    local tx = -c.ny
    local ty = c.nx
    local vt = vax * tx + vay * ty
    local raT = rax * ty - ray * tx
    local jt = -vt / (invA + raT * raT / a.iz)
    local maxF = friction * j
    if jt > maxF then
        jt = maxF
    end
    if jt < -maxF then
        jt = -maxF
    end
    a:applyImpulse(tx * jt, ty * jt, c.px, c.py)
    return j
end
return ____exports
 end,
["src.core.player.actions"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local __TS__ArraySlice = ____lualib.__TS__ArraySlice
local Map = ____lualib.Map
local __TS__New = ____lualib.__TS__New
local __TS__ArrayIndexOf = ____lualib.__TS__ArrayIndexOf
local __TS__ArraySplice = ____lualib.__TS__ArraySplice
local ____exports = {}
local ramp
function ramp(cur, target, maxDelta)
    if cur < target then
        return math.min(target, cur + maxDelta)
    end
    return math.max(target, cur - maxDelta)
end
local function A(id, label, category, kind, keys, pad, help)
    return {
        id = id,
        label = label,
        category = category,
        kind = kind,
        keys = keys,
        pad = pad,
        help = help
    }
end
____exports.ACTIONS = {
    A(
        "throttle",
        "Throttle",
        "Driving",
        "analog",
        {"KeyW", "ArrowUp"},
        {{button = 7}},
        "Accelerate (in reverse with automatic: brake)."
    ),
    A(
        "brake",
        "Brake / reverse",
        "Driving",
        "analog",
        {"KeyS", "ArrowDown"},
        {{button = 6}},
        "Brake. With automatic gearbox, hold at a standstill to reverse."
    ),
    A(
        "steer_left",
        "Steer left",
        "Driving",
        "analog",
        {"KeyA", "ArrowLeft"},
        {{axis = 0, dir = -1}},
        "Steer left."
    ),
    A(
        "steer_right",
        "Steer right",
        "Driving",
        "analog",
        {"KeyD", "ArrowRight"},
        {{axis = 0, dir = 1}},
        "Steer right."
    ),
    A(
        "handbrake",
        "Handbrake",
        "Driving",
        "hold",
        {"Space"},
        {{button = 0}},
        "Locks the rear wheels: parking, hairpins and drifts."
    ),
    A(
        "clutch",
        "Clutch",
        "Gearbox",
        "analog",
        {"KeyZ"},
        {{button = 4}},
        "Manual gearbox: press to shift or launch without stalling."
    ),
    A(
        "shift_up",
        "Shift up",
        "Gearbox",
        "press",
        {"ShiftLeft", "KeyR"},
        {{button = 1}},
        "Next gear (manual)."
    ),
    A(
        "shift_down",
        "Shift down",
        "Gearbox",
        "press",
        {"ControlLeft", "KeyF"},
        {{button = 2}},
        "Previous gear (manual)."
    ),
    A(
        "gear_reverse",
        "Select reverse",
        "Gearbox",
        "press",
        {"Backspace"},
        {},
        "Select R directly."
    ),
    A(
        "gear_neutral",
        "Select neutral",
        "Gearbox",
        "press",
        {"KeyN"},
        {},
        "Select N directly."
    ),
    A(
        "transmission_mode",
        "Auto / manual",
        "Gearbox",
        "press",
        {"KeyT"},
        {},
        "Toggle automatic and manual transmission."
    ),
    A(
        "horn",
        "Horn",
        "Signals & lights",
        "hold",
        {"KeyH"},
        {{button = 10}},
        "Tap for a beep, hold for a long blast (other drivers notice)."
    ),
    A(
        "indicator_left",
        "Left indicator",
        "Signals & lights",
        "press",
        {"KeyQ"},
        {{button = 14}},
        "Toggle left turn signal (auto-cancels after the turn)."
    ),
    A(
        "indicator_right",
        "Right indicator",
        "Signals & lights",
        "press",
        {"KeyE"},
        {{button = 15}},
        "Toggle right turn signal (auto-cancels after the turn)."
    ),
    A(
        "hazards",
        "Hazard lights",
        "Signals & lights",
        "press",
        {"KeyX"},
        {{button = 13}},
        "All four indicators. A quick blink after merging says thanks."
    ),
    A(
        "headlights",
        "Headlights",
        "Signals & lights",
        "press",
        {"KeyL"},
        {{button = 12}},
        "Cycle the light switch: OFF, AUTO, ON."
    ),
    A(
        "high_beam",
        "High beam",
        "Signals & lights",
        "press",
        {"KeyK"},
        {},
        "Toggle high beams (dazzles oncoming drivers)."
    ),
    A(
        "flash",
        "Flash high beams",
        "Signals & lights",
        "hold",
        {"KeyJ"},
        {{button = 5}},
        "Flash to pass / warn. Drivers ahead take it personally."
    ),
    A(
        "wipers",
        "Wipers",
        "Signals & lights",
        "press",
        {"KeyP"},
        {},
        "Cycle wipers: OFF, INT, LOW, HIGH."
    ),
    A(
        "siren",
        "Siren",
        "Signals & lights",
        "press",
        {"KeyO"},
        {},
        "Emergency vehicles only: cycle siren."
    ),
    A(
        "cruise_toggle",
        "Cruise control",
        "Comfort & assists",
        "press",
        {"KeyC"},
        {},
        "Set / cancel cruise at the current speed. Brake also cancels."
    ),
    A(
        "cruise_up",
        "Cruise +5",
        "Comfort & assists",
        "press",
        {"Equal", "NumpadAdd"},
        {},
        "Raise cruise speed by 5 km/h."
    ),
    A(
        "cruise_down",
        "Cruise -5",
        "Comfort & assists",
        "press",
        {"Minus", "NumpadSubtract"},
        {},
        "Lower cruise speed by 5 km/h."
    ),
    A(
        "limiter",
        "Speed limiter",
        "Comfort & assists",
        "press",
        {"KeyU"},
        {},
        "Cap speed at the posted limit."
    ),
    A(
        "engine",
        "Engine start / stop",
        "Comfort & assists",
        "press",
        {"KeyI"},
        {},
        "Turn the ignition. Needed after a stall."
    ),
    A(
        "seatbelt",
        "Seatbelt",
        "Comfort & assists",
        "press",
        {"KeyB"},
        {},
        "Buckle up. Police fine unbelted drivers."
    ),
    A(
        "toggle_abs",
        "ABS on/off",
        "Comfort & assists",
        "press",
        {"Digit1"},
        {},
        "Anti-lock brakes."
    ),
    A(
        "toggle_tcs",
        "Traction control on/off",
        "Comfort & assists",
        "press",
        {"Digit2"},
        {},
        "Traction control (off = burnouts)."
    ),
    A(
        "toggle_esc",
        "Stability control on/off",
        "Comfort & assists",
        "press",
        {"Digit3"},
        {},
        "Electronic stability control."
    ),
    A(
        "apologize",
        "Wave / sorry",
        "Social",
        "press",
        {"KeyG"},
        {{button = 3}},
        "Apologise to nearby drivers. Calms people you just upset."
    ),
    A(
        "rude_gesture",
        "Rude gesture",
        "Social",
        "press",
        {"KeyY"},
        {},
        "Insult nearby drivers. Expect consequences."
    ),
    A(
        "camera",
        "Change camera",
        "Camera",
        "press",
        {"KeyV"},
        {{button = 8}},
        "Chase, far chase, hood, cockpit, top-down, cinematic."
    ),
    A(
        "look_back",
        "Look back",
        "Camera",
        "hold",
        {"KeyM"},
        {{button = 11}},
        "Hold to look behind."
    ),
    A(
        "look_left",
        "Look left",
        "Camera",
        "hold",
        {"Comma"},
        {},
        "Hold to look left."
    ),
    A(
        "look_right",
        "Look right",
        "Camera",
        "hold",
        {"Period"},
        {},
        "Hold to look right."
    ),
    A(
        "reset_vehicle",
        "Reset vehicle",
        "System",
        "press",
        {"Backquote"},
        {},
        "Put the car back on the nearest road (repair costs money)."
    ),
    A(
        "pause",
        "Pause / menu",
        "System",
        "press",
        {"Escape"},
        {{button = 9}},
        "Pause and open the menu."
    ),
    A(
        "help",
        "Controls help",
        "System",
        "press",
        {"F1"},
        {},
        "Show all controls."
    ),
    A(
        "map",
        "Map",
        "System",
        "press",
        {"Tab"},
        {},
        "Toggle the big map."
    ),
    A(
        "debug",
        "Debug overlay",
        "System",
        "press",
        {"F3"},
        {},
        "Show AI moods, intentions and signal phases."
    ),
    A(
        "time_forward",
        "Skip time",
        "System",
        "press",
        {"BracketRight"},
        {},
        "Advance the clock by one hour."
    ),
    A(
        "weather_cycle",
        "Change weather",
        "System",
        "press",
        {"BracketLeft"},
        {},
        "Cycle clear, rain, storm, fog."
    )
}
function ____exports.actionDef(id)
    for ____, a in ipairs(____exports.ACTIONS) do
        if a.id == id then
            return a
        end
    end
    return ____exports.ACTIONS[1]
end
--- Collects raw keyboard/gamepad/direct input and turns it into ControlFrames
-- with keyboard smoothing (throttle ramps, speed-sensitive steering).
____exports.InputMapper = __TS__Class()
local InputMapper = ____exports.InputMapper
InputMapper.name = "InputMapper"
function InputMapper.prototype.____constructor(self)
    self.raw = __TS__New(Map)
    self.prev = __TS__New(Map)
    self.heldTime = __TS__New(Map)
    self.keysDown = __TS__New(Map)
    self.padValues = __TS__New(Map)
    self.direct = __TS__New(Map)
    self.pendingPress = {}
    self.keyIndex = __TS__New(Map)
    self.sThrottle = 0
    self.sBrake = 0
    self.sSteer = 0
    self.sClutch = 0
    self.analogFromPad = false
    self.bindings = __TS__New(Map)
    for ____, a in ipairs(____exports.ACTIONS) do
        self.bindings:set(
            a.id,
            {
                keys = __TS__ArraySlice(a.keys),
                pad = __TS__ArraySlice(a.pad)
            }
        )
    end
    self:rebuildIndex()
end
function InputMapper.prototype.rebuildIndex(self)
    self.keyIndex = __TS__New(Map)
    self.bindings:forEach(function(____, b, id)
        for ____, k in ipairs(b.keys) do
            local l = self.keyIndex:get(k)
            if l == nil then
                l = {}
                self.keyIndex:set(k, l)
            end
            l[#l + 1] = id
        end
    end)
end
function InputMapper.prototype.bindKeys(self, id, keys)
    local b = self.bindings:get(id)
    if b == nil then
        return
    end
    self.bindings:forEach(function(____, o)
        for ____, k in ipairs(keys) do
            local i = __TS__ArrayIndexOf(o.keys, k)
            if i >= 0 then
                __TS__ArraySplice(o.keys, i, 1)
            end
        end
    end)
    b.keys = __TS__ArraySlice(keys)
    self:rebuildIndex()
end
function InputMapper.prototype.bindPad(self, id, pad)
    local b = self.bindings:get(id)
    if b ~= nil then
        b.pad = __TS__ArraySlice(pad)
    end
end
function InputMapper.prototype.key(self, code, down)
    local ids = self.keyIndex:get(code)
    local was = self.keysDown:get(code) == true
    self.keysDown:set(code, down)
    if ids == nil then
        return false
    end
    if down and not was then
        for ____, id in ipairs(ids) do
            local ____self_pendingPress_0 = self.pendingPress
            ____self_pendingPress_0[#____self_pendingPress_0 + 1] = id
        end
    end
    if down then
        self.analogFromPad = false
    end
    return true
end
function InputMapper.prototype.gamepad(self, buttons, axes)
    local any = false
    for ____, def in ipairs(____exports.ACTIONS) do
        do
            local __continue30
            repeat
                local id = def.id
                local b = self.bindings:get(id)
                if b == nil then
                    __continue30 = true
                    break
                end
                local v = 0
                for ____, p in ipairs(b.pad) do
                    if p.button ~= nil and p.button < #buttons then
                        v = math.max(v, buttons[p.button + 1])
                    end
                    if p.axis ~= nil and p.axis < #axes then
                        local a = axes[p.axis + 1] * (p.dir or 1)
                        local dz = 0.12
                        if a > dz then
                            v = math.max(v, (a - dz) / (1 - dz))
                        end
                    end
                end
                local before = self.padValues:get(id) or 0
                if v > 0.5 and before <= 0.5 then
                    local ____self_pendingPress_1 = self.pendingPress
                    ____self_pendingPress_1[#____self_pendingPress_1 + 1] = id
                end
                if v > 0.05 then
                    any = true
                end
                self.padValues:set(id, v)
                __continue30 = true
            until true
            if not __continue30 then
                break
            end
        end
    end
    if any then
        self.analogFromPad = true
    end
end
function InputMapper.prototype.set(self, id, value)
    local before = self.direct:get(id) or 0
    if value > 0.5 and before <= 0.5 then
        local ____self_pendingPress_2 = self.pendingPress
        ____self_pendingPress_2[#____self_pendingPress_2 + 1] = id
    end
    self.direct:set(id, value)
end
function InputMapper.prototype.press(self, id)
    local ____self_pendingPress_3 = self.pendingPress
    ____self_pendingPress_3[#____self_pendingPress_3 + 1] = id
end
function InputMapper.prototype.releaseAll(self)
    self.keysDown = __TS__New(Map)
    self.direct = __TS__New(Map)
    self.padValues = __TS__New(Map)
end
function InputMapper.prototype.value(self, id)
    local v = self.direct:get(id) or 0
    v = math.max(
        v,
        self.padValues:get(id) or 0
    )
    local b = self.bindings:get(id)
    if b ~= nil then
        for ____, k in ipairs(b.keys) do
            if self.keysDown:get(k) == true then
                v = 1
            end
        end
    end
    return v
end
function InputMapper.prototype.frame(self, dt, speed)
    local pressed = {}
    local released = {}
    local held = {}
    for ____, id in ipairs(self.pendingPress) do
        if __TS__ArrayIndexOf(pressed, id) < 0 then
            pressed[#pressed + 1] = id
        end
    end
    self.pendingPress = {}
    for ____, a in ipairs(____exports.ACTIONS) do
        local v = self:value(a.id)
        local before = self.prev:get(a.id) or 0
        self.raw:set(a.id, v)
        if v > 0.5 then
            held[#held + 1] = a.id
            self.heldTime:set(
                a.id,
                (self.heldTime:get(a.id) or 0) + dt
            )
        elseif before > 0.5 then
            released[#released + 1] = {
                id = a.id,
                held = self.heldTime:get(a.id) or 0
            }
            self.heldTime:set(a.id, 0)
        end
        self.prev:set(a.id, v)
    end
    local thr = self.raw:get("throttle") or 0
    local brk = self.raw:get("brake") or 0
    local steer = (self.raw:get("steer_right") or 0) - (self.raw:get("steer_left") or 0)
    local clutch = self.raw:get("clutch") or 0
    if self.analogFromPad then
        self.sThrottle = thr
        self.sBrake = brk
        self.sSteer = self.sSteer + (steer - self.sSteer) * math.min(1, dt * 20)
        self.sClutch = clutch
    else
        self.sThrottle = ramp(self.sThrottle, thr, dt * (thr > self.sThrottle and 4 or 8))
        self.sBrake = ramp(self.sBrake, brk, dt * (brk > self.sBrake and 5 or 10))
        self.sClutch = ramp(self.sClutch, clutch, dt * 6)
        local speedFactor = 1 / (1 + math.abs(speed) / 18)
        local target = steer * (0.35 + 0.65 * speedFactor)
        local returning = math.abs(target) < math.abs(self.sSteer) or target * self.sSteer < 0
        self.sSteer = ramp(self.sSteer, target, dt * (returning and 5 or 2.2 + 1.8 * speedFactor))
    end
    return {
        throttle = self.sThrottle,
        brake = self.sBrake,
        steer = self.sSteer,
        handbrake = self.raw:get("handbrake") or 0,
        clutch = self.sClutch,
        pressed = pressed,
        released = released,
        held = held
    }
end
function InputMapper.prototype.isHeld(self, id)
    return (self.raw:get(id) or 0) > 0.5
end
function InputMapper.prototype.heldFor(self, id)
    return self.heldTime:get(id) or 0
end
return ____exports
 end,
["src.core.player.playerDriver"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local __TS__ArrayIndexOf = ____lualib.__TS__ArrayIndexOf
local ____exports = {}
local ____scalar = require("src.core.math.scalar")
local clamp = ____scalar.clamp
local KMH = ____scalar.KMH
--- Actions the simulation does not handle itself (cameras, menus): forwarded to the host.
____exports.HOST_ACTIONS = {
    "camera",
    "look_back",
    "look_left",
    "look_right",
    "pause",
    "help",
    "map",
    "debug",
    "time_forward",
    "weather_cycle",
    "reset_vehicle"
}
--- Turns the player's ControlFrame into vehicle inputs and operates every
-- piece of equipment: gearbox (auto/manual, clutch, stalls), cruise control,
-- speed limiter, ignition, lights, indicators, hazards, wipers, horn,
-- seatbelt and driver assists.
____exports.PlayerDriver = __TS__Class()
local PlayerDriver = ____exports.PlayerDriver
PlayerDriver.name = "PlayerDriver"
function PlayerDriver.prototype.____constructor(self, v)
    self.v = v
    self.settings = {clutchAssist = true, autoReverse = true, steerSensitivity = 1}
    self.cruise = {on = false, target = 0, integral = 0}
    self.limiterOn = false
    self.seatbelt = true
    self.parked = true
    self.reverseTimer = 0
    self.seatbeltChime = 0
    self.hostActions = {}
    self.gesture = nil
    self.notes = {}
    self.wasStalled = false
end
function PlayerDriver.prototype.gearText(self)
    local pt = self.v.pt
    if pt.mode == "automatic" then
        if self.parked then
            return "P"
        end
        local l = pt:gearLabel()
        return (l == "R" or l == "N") and l or "D" .. l
    end
    return pt:gearLabel()
end
function PlayerDriver.prototype.note(self, s)
    local ____self_notes_0 = self.notes
    ____self_notes_0[#____self_notes_0 + 1] = s
    if #self.notes > 8 then
        table.remove(self.notes, 1)
    end
end
function PlayerDriver.prototype.update(self, dt, f, ctx, speedLimit)
    local v = self.v
    local pt = v.pt
    local lights = v.lights
    self.gesture = nil
    for ____, id in ipairs(f.pressed) do
        self:onPress(id, ctx, speedLimit)
    end
    local hornHeld = __TS__ArrayIndexOf(f.held, "horn") >= 0
    if hornHeld and not lights.horn then
        lights.hornPresses = lights.hornPresses + 1
        ctx.events:emit({type = "horn", vehicle = v.id, long = false})
    end
    lights.horn = hornHeld
    local flashHeld = __TS__ArrayIndexOf(f.held, "flash") >= 0
    if flashHeld and not lights.flash then
        ctx.events:emit({type = "flash", vehicle = v.id})
    end
    lights.flash = flashHeld
    for ____, r in ipairs(f.released) do
        if r.id == "horn" and r.held > 0.9 then
            ctx.events:emit({type = "horn", vehicle = v.id, long = true})
        end
    end
    local throttle = f.throttle
    local brake = f.brake
    local u = v.dyn.u
    if pt.mode == "automatic" and self.parked then
        if pt.gear ~= 0 then
            pt:shiftTo(0)
        end
        if throttle > 0.15 and pt.running then
            self.parked = false
            pt:shiftTo(1)
        elseif brake > 0.15 and throttle < 0.05 and pt.running and self.settings.autoReverse then
            self.reverseTimer = self.reverseTimer + dt
            if self.reverseTimer > 0.4 then
                self.parked = false
                pt:shiftTo(-1)
                self.reverseTimer = 0
            end
        end
    elseif pt.mode == "automatic" then
        if not pt.running and math.abs(u) < 0.3 then
            self.parked = true
        end
        if self.settings.autoReverse then
            if pt.gear >= 1 then
                if brake > 0.15 and throttle < 0.05 and u < 0.4 then
                    self.reverseTimer = self.reverseTimer + dt
                    if self.reverseTimer > 0.3 and math.abs(u) < 0.3 then
                        pt:shiftTo(-1)
                        self.reverseTimer = 0
                    end
                else
                    self.reverseTimer = 0
                end
            elseif pt.gear == -1 then
                if throttle > 0.15 and brake < 0.05 and u > -0.4 then
                    self.reverseTimer = self.reverseTimer + dt
                    if self.reverseTimer > 0.2 and math.abs(u) < 0.3 then
                        pt:shiftTo(1)
                        self.reverseTimer = 0
                    end
                else
                    self.reverseTimer = 0
                end
                local t = throttle
                throttle = brake
                brake = t
            end
        end
        pt.clutchPedal = 0
    else
        if self.settings.clutchAssist and f.clutch < 0.05 then
            local wheelRpm = math.abs(v.dyn:drivenOmega() * pt:ratio(pt.gear) * v.spec.finalDrive) * 9.549
            local idle = v.spec.idleRpm
            if pt:shifting() then
                pt.clutchPedal = 1
            elseif pt.gear ~= 0 and wheelRpm < idle * 1.15 then
                pt.clutchPedal = clamp(1 - throttle * 1.6 - wheelRpm / idle * 0.5, 0, 1)
            else
                pt.clutchPedal = 0
            end
        else
            pt.clutchPedal = f.clutch
        end
        if pt.gear == -1 and u > 1.5 and throttle > 0.2 then
            pt:shiftTo(0)
        end
    end
    if pt.stalled and not self.wasStalled then
        ctx.events:emit({type = "stall", vehicle = v.id})
        self:note("Engine stalled — press Start (I)")
        self.cruise.on = false
    end
    self.wasStalled = pt.stalled
    local cr = self.cruise
    if cr.on then
        if f.brake > 0.08 or f.clutch > 0.5 or f.handbrake > 0.5 or not pt.running then
            cr.on = false
            self:note("Cruise cancelled")
        elseif pt.gear >= 1 then
            local err = cr.target - u
            cr.integral = clamp(cr.integral + err * dt, -8, 8)
            local cmd = clamp(0.18 * err + 0.06 * cr.integral + 0.15, 0, 1)
            throttle = math.max(throttle, cmd)
        end
    end
    pt.limiter = self.limiterOn and math.max(speedLimit, 5) or 0
    v.inputs.throttle = (pt.running or pt.cranking > 0) and throttle or 0
    v.inputs.brake = brake
    v.inputs.handbrake = self.parked and pt.mode == "automatic" and 1 or f.handbrake
    local speedFactor = 1 / (1 + math.abs(u) / 35)
    v.steerTarget = -f.steer * v.spec.maxSteer * speedFactor * self.settings.steerSensitivity
    lights.brake = v.inputs.brake > 0.05 and 1 or 0
    lights.reverse = pt.gear == -1
    lights.dark = ctx.env.dark
    if not self.seatbelt and math.abs(u) > 3 then
        self.seatbeltChime = self.seatbeltChime - dt
        if self.seatbeltChime <= 0 then
            self.seatbeltChime = 6
            self:note("Fasten seatbelt (B)")
        end
    end
end
function PlayerDriver.prototype.onPress(self, id, ctx, speedLimit)
    local v = self.v
    local pt = v.pt
    local lights = v.lights
    repeat
        local ____switch46 = id
        local ____cond46 = ____switch46 == "shift_up"
        if ____cond46 then
            if pt.mode == "manual" and pt:shiftUp() then
                ctx.events:emit({
                    type = "shift",
                    vehicle = v.id,
                    gear = pt:gearLabel()
                })
            elseif pt.mode == "automatic" and pt.gear <= 0 then
                self.parked = false
                pt:shiftTo(1)
            end
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "shift_down"
        if ____cond46 then
            if pt.mode == "manual" and pt:shiftDown() then
                ctx.events:emit({
                    type = "shift",
                    vehicle = v.id,
                    gear = pt:gearLabel()
                })
            elseif pt.mode == "automatic" and math.abs(v.dyn.u) < 0.5 then
                self.parked = true
            end
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "gear_reverse"
        if ____cond46 then
            if math.abs(v.dyn.u) < 1.5 then
                self.parked = false
                pt:shiftTo(-1)
            end
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "gear_neutral"
        if ____cond46 then
            self.parked = false
            pt:shiftTo(0)
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "transmission_mode"
        if ____cond46 then
            pt.mode = pt.mode == "automatic" and "manual" or "automatic"
            self.parked = false
            if pt.mode == "automatic" and pt.gear == 0 then
                pt:shiftTo(1)
            end
            self:note(pt.mode == "manual" and "Manual transmission" or "Automatic transmission")
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "indicator_left"
        if ____cond46 then
            lights:toggleIndicator("left")
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "indicator_right"
        if ____cond46 then
            lights:toggleIndicator("right")
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "hazards"
        if ____cond46 then
            lights.hazard = not lights.hazard
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "headlights"
        if ____cond46 then
            lights:cycleHeadlights()
            self:note("Lights: " .. string.upper(lights.headlightSwitch))
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "high_beam"
        if ____cond46 then
            lights.highBeam = not lights.highBeam
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "wipers"
        if ____cond46 then
            lights:cycleWipers()
            self:note("Wipers: " .. string.upper(lights.wipers))
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "siren"
        if ____cond46 then
            if v.spec.emergency then
                lights:cycleSiren()
            end
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "cruise_toggle"
        if ____cond46 then
            if self.cruise.on then
                self.cruise.on = false
                self:note("Cruise off")
            elseif v.dyn.u > 30 * KMH * 0.5 and pt.running then
                self.cruise.on = true
                self.cruise.target = v.dyn.u
                self.cruise.integral = 0
                self:note(("Cruise set " .. tostring(math.floor(self.cruise.target * 3.6 + 0.5))) .. " km/h")
            else
                self:note("Cruise needs at least 15 km/h")
            end
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "cruise_up"
        if ____cond46 then
            if self.cruise.on then
                local ____self_cruise_1, ____target_2 = self.cruise, "target"
                ____self_cruise_1[____target_2] = ____self_cruise_1[____target_2] + 5 * KMH
                self:note(("Cruise " .. tostring(math.floor(self.cruise.target * 3.6 + 0.5))) .. " km/h")
            end
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "cruise_down"
        if ____cond46 then
            if self.cruise.on then
                self.cruise.target = math.max(15 * KMH, self.cruise.target - 5 * KMH)
                self:note(("Cruise " .. tostring(math.floor(self.cruise.target * 3.6 + 0.5))) .. " km/h")
            end
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "limiter"
        if ____cond46 then
            self.limiterOn = not self.limiterOn
            self:note(self.limiterOn and ("Limiter " .. tostring(math.floor(speedLimit * 3.6 + 0.5))) .. " km/h" or "Limiter off")
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "engine"
        if ____cond46 then
            if pt.running then
                pt:stop()
                ctx.events:emit({type = "engine", vehicle = v.id, on = false})
                self:note("Engine off")
            else
                pt:start()
                ctx.events:emit({type = "engine", vehicle = v.id, on = true})
            end
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "seatbelt"
        if ____cond46 then
            self.seatbelt = not self.seatbelt
            self:note(self.seatbelt and "Seatbelt fastened" or "Seatbelt unfastened")
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "toggle_abs"
        if ____cond46 then
            v.assists.abs = not v.assists.abs
            self:note("ABS " .. (v.assists.abs and "on" or "off"))
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "toggle_tcs"
        if ____cond46 then
            v.assists.tcs = not v.assists.tcs
            self:note("Traction control " .. (v.assists.tcs and "on" or "off"))
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "toggle_esc"
        if ____cond46 then
            v.assists.esc = not v.assists.esc
            self:note("Stability control " .. (v.assists.esc and "on" or "off"))
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "apologize"
        if ____cond46 then
            self.gesture = "wave"
            break
        end
        ____cond46 = ____cond46 or ____switch46 == "rude_gesture"
        if ____cond46 then
            self.gesture = "rude"
            break
        end
        do
            if __TS__ArrayIndexOf(____exports.HOST_ACTIONS, id) >= 0 then
                local ____self_hostActions_3 = self.hostActions
                ____self_hostActions_3[#____self_hostActions_3 + 1] = id
            end
        end
    until true
end
return ____exports
 end,
["src.core.road.controls"] = function(...) 
--[[ Generated with https://github.com/TypeScriptToLua/TypeScriptToLua ]]
local ____exports = {}
local KIND_RANK
--- Importance of a road for right-of-way decisions.
function ____exports.roadRank(r)
    return KIND_RANK[r.kind] * 10 + r.opts.lanesFwd + r.opts.lanesBack
end
function ____exports.assignNode(n)
    local best = -1
    for ____, r in ipairs(n.roads) do
        best = math.max(
            best,
            ____exports.roadRank(r)
        )
    end
    for ____, ap in ipairs(n.approaches) do
        local isMajor = ____exports.roadRank(ap.road) >= best
        repeat
            local ____switch10 = n.control
            local ____cond10 = ____switch10 == "signal"
            if ____cond10 then
                ap.sign = "signal"
                ap.major = isMajor
                break
            end
            ____cond10 = ____cond10 or ____switch10 == "allway_stop"
            if ____cond10 then
                ap.sign = "stop"
                ap.major = false
                break
            end
            ____cond10 = ____cond10 or ____switch10 == "twoway_stop"
            if ____cond10 then
                ap.sign = isMajor and "none" or "stop"
                ap.major = isMajor
                break
            end
            ____cond10 = ____cond10 or ____switch10 == "yield"
            if ____cond10 then
                ap.sign = isMajor and "none" or "yield"
                ap.major = isMajor
                break
            end
            do
                ap.sign = "none"
                ap.major = true
            end
        until true
    end
    if n.control == "twoway_stop" or n.control == "yield" then
        local minors = 0
        for ____, ap in ipairs(n.approaches) do
            if not ap.major then
                minors = minors + 1
            end
        end
        if minors == 0 and #n.approaches >= 3 then
            n.control = "allway_stop"
            for ____, ap in ipairs(n.approaches) do
                ap.sign = "stop"
                ap.major = false
            end
        end
    end
end
KIND_RANK = {
    ring = 5,
    boulevard = 5,
    avenue = 4,
    oneway = 3,
    street = 2,
    alley = 1
}
--- Translate each node's control type into per-approach signs and major-road
-- flags. Call after Network.finalize().
function ____exports.assignApproachSigns(net)
    for ____, n in ipairs(net.nodes) do
        ____exports.assignNode(n)
    end
end
return ____exports
 end,
["src.core.road.cityBuilder"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__New = ____lualib.__TS__New
local ____exports = {}
local ____rng = require("src.core.math.rng")
local Rng = ____rng.Rng
local ____network = require("src.core.road.network")
local Network = ____network.Network
local ____controls = require("src.core.road.controls")
local assignApproachSigns = ____controls.assignApproachSigns
local STREET_NAMES = {
    "Kestrel",
    "Ashgrove",
    "Ozone",
    "Delamain",
    "Ironside",
    "Halcyon",
    "Corvid",
    "Lumen",
    "Tidewater",
    "Saffron",
    "Quartz",
    "Juniper",
    "Mercury",
    "Bellwether"
}
local AVENUE_NAMES = {"Meridian Ave", "Vanta Ave", "Arcadia Ave", "Helix Ave"}
local function lineSpecs(count, rng, axis)
    local specs = {}
    local oneWayToggle = rng:chance(0.5)
    local avenueIdx = 0
    local streetIdx = axis == "col" and 0 or 7
    do
        local i = 0
        while i < count do
            local s
            if i == 0 or i == count - 1 then
                s = {
                    kind = "ring",
                    inc = 2,
                    dec = 2,
                    speed = 70,
                    name = "Perimeter Blvd",
                    width = 0
                }
            elseif i == math.floor(count / 2) or count > 8 and i == 2 then
                s = {
                    kind = "avenue",
                    inc = 3,
                    dec = 3,
                    speed = 60,
                    name = AVENUE_NAMES[avenueIdx + 1],
                    width = 0
                }
                avenueIdx = avenueIdx + 1 < #AVENUE_NAMES and avenueIdx + 1 or 0
            elseif rng:chance(0.35) then
                oneWayToggle = not oneWayToggle
                local lanes = rng:chance(0.5) and 2 or 3
                s = {
                    kind = "oneway",
                    inc = oneWayToggle and lanes or 0,
                    dec = oneWayToggle and 0 or lanes,
                    speed = 50,
                    name = STREET_NAMES[streetIdx + 1] .. " St",
                    width = 0
                }
                streetIdx = streetIdx + 1 < #STREET_NAMES and streetIdx + 1 or 0
            else
                local lanes = rng:chance(0.3) and 2 or 1
                s = {
                    kind = "street",
                    inc = lanes,
                    dec = lanes,
                    speed = lanes == 2 and 50 or 40,
                    name = STREET_NAMES[streetIdx + 1] .. " St",
                    width = 0
                }
                streetIdx = streetIdx + 1 < #STREET_NAMES and streetIdx + 1 or 0
            end
            local median = s.inc > 0 and s.dec > 0 and (s.kind == "avenue" and 2 or (s.kind == "ring" and 1.5 or 0.3)) or 0
            s.width = (s.inc + s.dec) * 3.5 + median
            specs[#specs + 1] = s
            i = i + 1
        end
    end
    return specs
end
local function chooseControl(a, b, rng, roads)
    if roads <= 2 then
        return "none"
    end
    local function big(s)
        return s.kind == "avenue" or s.kind == "ring"
    end
    if a.kind == "ring" and b.kind == "ring" then
        return "none"
    end
    if big(a) and big(b) then
        return "signal"
    end
    if a.kind == "avenue" or b.kind == "avenue" then
        return "signal"
    end
    if a.kind == "ring" or b.kind == "ring" then
        return rng:chance(0.4) and "signal" or "twoway_stop"
    end
    if a.kind == "oneway" and b.kind == "oneway" then
        return rng:chance(0.6) and "signal" or "allway_stop"
    end
    if a.kind == "oneway" or b.kind == "oneway" then
        return rng:chance(0.5) and "twoway_stop" or "signal"
    end
    return rng:chance(0.65) and "allway_stop" or "twoway_stop"
end
--- Procedural grid city: a perimeter boulevard, multi-lane avenues with
-- protected left turns, one-way streets and small streets with stop signs.
function ____exports.buildCity(opts)
    local rng = __TS__New(Rng, opts.seed or 2077)
    local cols = opts.cols or 7
    local rows = opts.rows or 6
    local bw = opts.blockW or 110
    local bh = opts.blockH or 95
    local xs = {}
    local ys = {}
    local x = 0
    do
        local i = 0
        while i < cols do
            xs[#xs + 1] = x
            x = x + bw * rng:range(0.8, 1.2)
            i = i + 1
        end
    end
    local y = 0
    do
        local j = 0
        while j < rows do
            ys[#ys + 1] = y
            y = y + bh * rng:range(0.8, 1.2)
            j = j + 1
        end
    end
    local cx = xs[cols] / 2
    local cy = ys[rows] / 2
    do
        local i = 0
        while i < cols do
            xs[i + 1] = math.floor(xs[i + 1] - cx)
            i = i + 1
        end
    end
    do
        local j = 0
        while j < rows do
            ys[j + 1] = math.floor(ys[j + 1] - cy)
            j = j + 1
        end
    end
    local colSpec = lineSpecs(cols, rng, "col")
    local rowSpec = lineSpecs(rows, rng, "row")
    local net = __TS__New(Network)
    local grid = {}
    do
        local i = 0
        while i < cols do
            local colNodes = {}
            do
                local j = 0
                while j < rows do
                    local onEdgeX = i == 0 or i == cols - 1
                    local onEdgeY = j == 0 or j == rows - 1
                    local degree = onEdgeX and onEdgeY and 2 or ((onEdgeX or onEdgeY) and 3 or 4)
                    local n = net:addNode(
                        xs[i + 1],
                        ys[j + 1],
                        chooseControl(colSpec[i + 1], rowSpec[j + 1], rng, degree)
                    )
                    n.name = (colSpec[i + 1].name .. " & ") .. rowSpec[j + 1].name
                    colNodes[#colNodes + 1] = n
                    j = j + 1
                end
            end
            grid[#grid + 1] = colNodes
            i = i + 1
        end
    end
    local function mk(a, b, s)
        local o = {
            lanesFwd = s.inc,
            lanesBack = s.dec,
            speedLimitKmh = s.speed,
            kind = s.kind,
            name = s.name,
            medianWidth = s.inc > 0 and s.dec > 0 and (s.kind == "avenue" and 2 or (s.kind == "ring" and 1.5 or 0.3)) or 0,
            sidewalkWidth = s.kind == "ring" and 4 or 3
        }
        return net:addRoad(a, b, o)
    end
    do
        local i = 0
        while i < cols do
            do
                local j = 0
                while j < rows - 1 do
                    mk(grid[i + 1][j + 1], grid[i + 1][j + 1 + 1], colSpec[i + 1])
                    j = j + 1
                end
            end
            i = i + 1
        end
    end
    do
        local j = 0
        while j < rows do
            do
                local i = 0
                while i < cols - 1 do
                    mk(grid[i + 1][j + 1], grid[i + 1 + 1][j + 1], rowSpec[j + 1])
                    i = i + 1
                end
            end
            j = j + 1
        end
    end
    net:finalize()
    assignApproachSigns(net)
    for ____, n in ipairs(net.nodes) do
        if n.control == "signal" and rng:chance(0.3) then
            n.redLightCamera = true
        end
    end
    local speedCameras = {}
    for ____, r in ipairs(net.roads) do
        if (r.kind == "avenue" or r.kind == "ring") and rng:chance(0.15) and r.centerPath.length > 60 then
            r.speedCamera = true
            speedCameras[#speedCameras + 1] = {
                roadId = r.id,
                s = r.centerPath.length / 2,
                forward = rng:chance(0.5)
            }
        end
    end
    local buildings = {}
    local maxD = math.sqrt(cx * cx + cy * cy)
    do
        local i = 0
        while i < cols - 1 do
            do
                local j = 0
                while j < rows - 1 do
                    do
                        local __continue36
                        repeat
                            local x0 = xs[i + 1] + colSpec[i + 1].width / 2 + 3.5
                            local x1 = xs[i + 1 + 1] - colSpec[i + 1 + 1].width / 2 - 3.5
                            local y0 = ys[j + 1] + rowSpec[j + 1].width / 2 + 3.5
                            local y1 = ys[j + 1 + 1] - rowSpec[j + 1 + 1].width / 2 - 3.5
                            if x1 - x0 < 10 or y1 - y0 < 10 then
                                __continue36 = true
                                break
                            end
                            local mx = (x0 + x1) / 2
                            local my = (y0 + y1) / 2
                            local centrality = 1 - math.sqrt(mx * mx + my * my) / maxD
                            if rng:chance(0.08) then
                                buildings[#buildings + 1] = {
                                    x = mx,
                                    y = my,
                                    w = x1 - x0,
                                    h = y1 - y0,
                                    height = 0,
                                    kind = rng:chance(0.5) and "park" or "plaza",
                                    hue = rng:next()
                                }
                                __continue36 = true
                                break
                            end
                            local nx = x1 - x0 > 70 and 2 or 1
                            local ny = y1 - y0 > 60 and 2 or 1
                            local gap = 5
                            local cw = (x1 - x0 - gap * (nx - 1)) / nx
                            local ch = (y1 - y0 - gap * (ny - 1)) / ny
                            do
                                local a = 0
                                while a < nx do
                                    do
                                        local b = 0
                                        while b < ny do
                                            local bx = x0 + a * (cw + gap) + cw / 2
                                            local by = y0 + b * (ch + gap) + ch / 2
                                            local tall = rng:chance(0.25 + centrality * 0.5)
                                            local height = tall and rng:range(45, 90) + centrality * 120 or rng:range(10, 30)
                                            buildings[#buildings + 1] = {
                                                x = bx,
                                                y = by,
                                                w = cw * rng:range(0.82, 1),
                                                h = ch * rng:range(0.82, 1),
                                                height = height,
                                                kind = tall and "tower" or (rng:chance(0.1) and "garage" or "block"),
                                                hue = rng:next()
                                            }
                                            b = b + 1
                                        end
                                    end
                                    a = a + 1
                                end
                            end
                            __continue36 = true
                        until true
                        if not __continue36 then
                            break
                        end
                    end
                    j = j + 1
                end
            end
            i = i + 1
        end
    end
    return {
        network = net,
        buildings = buildings,
        speedCameras = speedCameras,
        xs = xs,
        ys = ys,
        name = "Gridlock City"
    }
end
return ____exports
 end,
["src.core.world.spawner"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local __TS__ArrayMap = ____lualib.__TS__ArrayMap
local __TS__New = ____lualib.__TS__New
local ____exports = {}
local ____driver = require("src.core.ai.driver")
local AIDriver = ____driver.AIDriver
local ____personality = require("src.core.ai.personality")
local samplePersonality = ____personality.samplePersonality
local ____specs = require("src.core.vehicle.specs")
local CIVILIAN_CLASSES = ____specs.CIVILIAN_CLASSES
local SPECS = ____specs.SPECS
local ____vehicle = require("src.core.vehicle.vehicle")
local Vehicle = ____vehicle.Vehicle
--- Keeps the population near its target: spawns civilians (weighted vehicle
-- mix, sampled personalities) and patrol cars away from the player, removes
-- long-dead wrecks and surplus vehicles far away.
____exports.Spawner = __TS__Class()
local Spawner = ____exports.Spawner
Spawner.name = "Spawner"
function Spawner.prototype.____constructor(self, host, opts)
    self.host = host
    self.opts = opts
    self.timer = 0
end
function Spawner.prototype.pickClass(self)
    local entries = __TS__ArrayMap(
        CIVILIAN_CLASSES,
        function(____, c) return {item = c, weight = SPECS[c].spawnWeight} end
    )
    return self.host.rng:weighted(entries)
end
function Spawner.prototype.create(self, cls, role, lane, s, speed, archetype)
    local h = self.host
    local spec = SPECS[cls]
    local color = h.rng:pick(spec.colors)
    local v = __TS__New(
        Vehicle,
        h.nextId(),
        spec,
        color,
        role
    )
    local arch = archetype
    if arch == nil then
        if role == "police" or role == "ambulance" or cls == "bus" or cls == "truck" then
            arch = "professional"
        elseif cls == "taxi" then
            arch = h.rng:chance(0.5) and "professional" or "speeder"
        elseif cls == "sports" and h.rng:chance(0.4) then
            arch = "joyrider"
        end
    end
    local p = samplePersonality(h.rng, arch)
    if role == "police" then
        p.lawfulness = 1
        p.aggression = 0.3
        p.temper = 0.3
    end
    local d = __TS__New(
        AIDriver,
        v,
        p,
        math.floor(h.rng:next() * 1000000000) + 1
    )
    v:placeOnSegment(lane, s, speed)
    v.lights.headlightSwitch = h.rng:chance(0.95) and "auto" or "off"
    h.add(v, d)
    return v
end
function Spawner.prototype.findSpot(self, minD, maxD)
    local h = self.host
    local pp = h.playerPos()
    do
        local attempt = 0
        while attempt < 25 do
            do
                local __continue12
                repeat
                    local lane = h.lanes[math.floor(h.rng:next() * #h.lanes) + 1]
                    if lane.length < 30 then
                        __continue12 = true
                        break
                    end
                    local s = h.rng:range(10, lane.length - 15)
                    local pt = {x = 0, y = 0, heading = 0}
                    lane.path:sample(s, pt)
                    if pp ~= nil then
                        local d = math.sqrt((pt.x - pp.x) * (pt.x - pp.x) + (pt.y - pp.y) * (pt.y - pp.y))
                        if d < minD or d > maxD then
                            __continue12 = true
                            break
                        end
                    end
                    if not h.laneFree(lane, s, 14) then
                        __continue12 = true
                        break
                    end
                    return {lane = lane, s = s}
                until true
                if not __continue12 then
                    break
                end
            end
            attempt = attempt + 1
        end
    end
    return nil
end
function Spawner.prototype.populate(self, density)
    local n = math.floor(self.opts.civilians * density)
    do
        local i = 0
        while i < n do
            do
                local __continue18
                repeat
                    local spot = self:findSpot(25, math.huge)
                    if spot == nil then
                        __continue18 = true
                        break
                    end
                    self:create(
                        self:pickClass(),
                        "civilian",
                        spot.lane,
                        spot.s,
                        spot.lane.speedLimit * 0.6
                    )
                    __continue18 = true
                until true
                if not __continue18 then
                    break
                end
            end
            i = i + 1
        end
    end
    do
        local i = 0
        while i < self.opts.police do
            local spot = self:findSpot(60, math.huge)
            if spot ~= nil then
                self:create(
                    "police",
                    "police",
                    spot.lane,
                    spot.s,
                    5
                )
            end
            i = i + 1
        end
    end
end
function Spawner.prototype.update(self, dt, density)
    self.timer = self.timer - dt
    if self.timer > 0 then
        return
    end
    self.timer = 0.5
    local h = self.host
    local pp = h.playerPos()
    local civ = 0
    local police = 0
    for ____, v in ipairs(h.vehicles) do
        if v.role == "police" then
            police = police + 1
        elseif v.role ~= "player" then
            civ = civ + 1
        end
    end
    local target = math.floor(self.opts.civilians * density)
    for ____, v in ipairs(h.vehicles) do
        do
            local __continue28
            repeat
                if v.isPlayer or h.busyUnit(v) then
                    __continue28 = true
                    break
                end
                local ____temp_0
                if pp == nil then
                    ____temp_0 = true
                else
                    ____temp_0 = (v.x - pp.x) * (v.x - pp.x) + (v.y - pp.y) * (v.y - pp.y) > self.opts.minDistance * self.opts.minDistance
                end
                local far = ____temp_0
                if v.wrecked then
                    v.despawnTimer = v.despawnTimer + 0.5
                end
                if v.despawnTimer > 45 and far or v.despawnTimer > 240 then
                    h.remove(v)
                    return
                end
                __continue28 = true
            until true
            if not __continue28 then
                break
            end
        end
    end
    if civ < target then
        local spot = self:findSpot(self.opts.minDistance, math.huge)
        if spot ~= nil then
            self:create(
                self:pickClass(),
                "civilian",
                spot.lane,
                spot.s,
                spot.lane.speedLimit * 0.7
            )
        end
    elseif civ > target * 1.15 + 2 then
        local surplus = nil
        for ____, v in ipairs(h.vehicles) do
            local near = pp ~= nil and (v.x - pp.x) * (v.x - pp.x) + (v.y - pp.y) * (v.y - pp.y) < 150 * 150
            if surplus == nil and v.role == "civilian" and v.mode == "lane" and not near then
                surplus = v
            end
        end
        if surplus ~= nil then
            h.remove(surplus)
        end
    end
    if police < self.opts.police then
        local spot = self:findSpot(self.opts.minDistance, math.huge)
        if spot ~= nil then
            self:create(
                "police",
                "police",
                spot.lane,
                spot.s,
                5
            )
        end
    end
end
return ____exports
 end,
["src.core.world.world"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__Class = ____lualib.__TS__Class
local __TS__New = ____lualib.__TS__New
local __TS__ObjectAssign = ____lualib.__TS__ObjectAssign
local __TS__ArrayIndexOf = ____lualib.__TS__ArrayIndexOf
local Map = ____lualib.Map
local __TS__ArraySplice = ____lualib.__TS__ArraySplice
local __TS__ArrayEvery = ____lualib.__TS__ArrayEvery
local __TS__ArraySort = ____lualib.__TS__ArraySort
local ____exports = {}
local lanePairCanCollide, setBox
local ____driver = require("src.core.ai.driver")
local hornVictims = ____driver.hornVictims
local ____speech = require("src.core.ai.speech")
local APOLOGY_ACCEPTED = ____speech.APOLOGY_ACCEPTED
local APOLOGY_REJECTED = ____speech.APOLOGY_REJECTED
local ____violations = require("src.core.law.violations")
local DrivingRecord = ____violations.DrivingRecord
local VIOLATIONS = ____violations.VIOLATIONS
local ____playerMonitor = require("src.core.law.playerMonitor")
local PlayerMonitor = ____playerMonitor.PlayerMonitor
local ____police = require("src.core.law.police")
local PoliceSystem = ____police.PoliceSystem
local ____rng = require("src.core.math.rng")
local Rng = ____rng.Rng
local ____scalar = require("src.core.math.scalar")
local angleDiff = ____scalar.angleDiff
local clamp = ____scalar.clamp
local DEG = ____scalar.DEG
local hypot = ____scalar.hypot
local mod = ____scalar.mod
local ____collision = require("src.core.physics.collision")
local boxContact = ____collision.boxContact
local resolveBodies = ____collision.resolveBodies
local resolveStatic = ____collision.resolveStatic
local ____actions = require("src.core.player.actions")
local InputMapper = ____actions.InputMapper
local ____playerDriver = require("src.core.player.playerDriver")
local PlayerDriver = ____playerDriver.PlayerDriver
local ____cityBuilder = require("src.core.road.cityBuilder")
local buildCity = ____cityBuilder.buildCity
local ____network = require("src.core.road.network")
local addOccupant = ____network.addOccupant
local ____router = require("src.core.road.router")
local Router = ____router.Router
local ____signalController = require("src.core.signals.signalController")
local SignalController = ____signalController.SignalController
local ____events = require("src.core.util.events")
local EventBus = ____events.EventBus
local ____spatialHash = require("src.core.util.spatialHash")
local SpatialHash = ____spatialHash.SpatialHash
local ____specs = require("src.core.vehicle.specs")
local SPECS = ____specs.SPECS
local ____vehicle = require("src.core.vehicle.vehicle")
local Vehicle = ____vehicle.Vehicle
local ____environment = require("src.core.world.environment")
local Environment = ____environment.Environment
local ____spawner = require("src.core.world.spawner")
local Spawner = ____spawner.Spawner
function lanePairCanCollide(a, b)
    local sa = a.seg
    local sb = b.seg
    if sa == nil or sb == nil then
        return false
    end
    if sa == sb or a.prevSeg == sb or b.prevSeg == sa then
        return math.abs(angleDiff(a.heading, b.heading)) < 35 * DEG
    end
    if sa.kind == "connector" and sb.kind == "connector" then
        local ca = sa
        for ____, z in ipairs(ca.conflicts) do
            if z.other == sb then
                return true
            end
        end
    end
    return false
end
function setBox(b, v, shrink)
    b.x = v.x
    b.y = v.y
    b.heading = v.heading
    b.hl = v.halfLen * shrink
    b.hw = v.halfWid * shrink
end
local contact = {
    nx = 0,
    ny = 0,
    depth = 0,
    px = 0,
    py = 0
}
local boxA = {
    x = 0,
    y = 0,
    heading = 0,
    hl = 0,
    hw = 0
}
local boxB = {
    x = 0,
    y = 0,
    heading = 0,
    hl = 0,
    hw = 0
}
--- The whole simulation. Hosts call update(realDt) every frame (it runs fixed
-- steps internally), feed input through `input`, and read vehicles, signals,
-- HUD state and drained events for rendering/audio.
____exports.World = __TS__Class()
local World = ____exports.World
World.name = "World"
function World.prototype.____constructor(self, opts)
    if opts == nil then
        opts = {}
    end
    self.time = 0
    self.signals = {}
    self.vehicles = {}
    self.events = __TS__New(EventBus)
    self.player = nil
    self.playerDriver = nil
    self.input = __TS__New(InputMapper)
    self.monitor = __TS__New(PlayerMonitor)
    self.record = __TS__New(DrivingRecord)
    self.police = __TS__New(PoliceSystem)
    self.stats = {
        collisions = 0,
        rageEpisodes = 0,
        horns = 0,
        redLightRunners = 0,
        spawned = 0
    }
    self.timeScale = 1
    self.hostActions = {}
    self.obstacles = {}
    self.drivers = __TS__New(Map)
    self.byId = __TS__New(Map)
    self.allWayQueues = __TS__New(Map)
    self.obstacleHash = __TS__New(SpatialHash, 30)
    self.vehicleHash = __TS__New(SpatialHash, 12)
    self.idCounter = 1
    self.accumulator = 0
    self.laneHits = {}
    self.connHits = {}
    self.queryBuf = {}
    self.obstacleBuf = {}
    self.flashMemo = __TS__New(Map)
    self.ambulance = nil
    self.ambulanceTarget = nil
    self.playerTailgate = __TS__New(Map)
    self.nearMissMemo = __TS__New(Map)
    self.nightFlashing = false
    self.cleanupTimer = 0
    self.demandFn = function(c)
        local lane = c.from
        local n = 0
        do
            local i = 0
            while i < lane.occCount do
                do
                    local __continue14
                    repeat
                        local e = lane.occ[i + 1]
                        if e.s < lane.length - 60 then
                            __continue14 = true
                            break
                        end
                        if e.occ.intentConnector == c.id or e.phantom and e.occ.intentConnector == -1 then
                            n = n + 1
                        end
                        __continue14 = true
                    until true
                    if not __continue14 then
                        break
                    end
                end
                i = i + 1
            end
        end
        do
            local i = 0
            while i < c.occCount do
                if c.occ[i + 1].v < 2 then
                    n = n + 1
                end
                i = i + 1
            end
        end
        return n
    end
    self.observations = {}
    local seed = opts.seed or 2077
    self.rng = __TS__New(Rng, seed)
    self.fixedDt = opts.fixedDt or 1 / 60
    self.city = buildCity(__TS__ObjectAssign({seed = seed}, opts.city or ({})))
    self.net = self.city.network
    self.router = __TS__New(Router, self.net)
    self.env = __TS__New(Environment, seed + 1, opts.startHour or 19.5)
    self:buildSignals()
    self:buildObstacles()
    local host = {
        rng = self.rng,
        vehicles = self.vehicles,
        lanes = self.net.lanes,
        playerPos = function() return self.player ~= nil and ({x = self.player.x, y = self.player.y}) or nil end,
        add = function(v, d) return self:addVehicle(v, d) end,
        remove = function(v) return self:removeVehicle(v) end,
        nextId = function()
            local ____self_0, ____idCounter_1 = self, "idCounter"
            local ____self_idCounter_2 = ____self_0[____idCounter_1]
            ____self_0[____idCounter_1] = ____self_idCounter_2 + 1
            return ____self_idCounter_2
        end,
        driverOf = function(v) return self.drivers:get(v.id) end,
        laneFree = function(lane, s, clear) return self:laneFree(lane, s, clear) end,
        busyUnit = function(v) return __TS__ArrayIndexOf(self.police.units, v) >= 0 or v == self.ambulance end
    }
    self.spawner = __TS__New(Spawner, host, {civilians = opts.traffic or 110, police = opts.police or 4, minDistance = 90})
    if opts.noPlayer ~= true then
        self:spawnPlayer(opts.playerVehicle or "sports")
    end
    self.spawner:populate(self.env.trafficDensity)
    self.police.spawnUnit = function(x, y)
        local spot = self.spawner:findSpot(120, 400)
        local ____ = x
        local ____ = y
        if spot == nil then
            return nil
        end
        return self.spawner:create(
            "police",
            "police",
            spot.lane,
            spot.s,
            8
        )
    end
    self:rebuildOccupancy()
end
function World.prototype.buildSignals(self)
    local coord = 0
    for ____, n in ipairs(self.net.nodes) do
        do
            local __continue20
            repeat
                if n.control ~= "signal" then
                    __continue20 = true
                    break
                end
                local offset = math.max(0, (n.x - self.net.minX) / 13.9)
                local mode = self.rng:chance(0.75) and "actuated" or "fixed"
                local sig = __TS__New(
                    SignalController,
                    #self.signals,
                    n,
                    {
                        mode = mode,
                        offset = mode == "fixed" and offset or self.rng:range(0, 30),
                        allowRightOnRed = self.rng:chance(0.85)
                    }
                )
                sig.onChange = function(c) return self.events:emit({type = "signal", node = c.nodeId, phase = c.phase, state = c.state}) end
                n.signalId = #self.signals
                local ____self_signals_3 = self.signals
                ____self_signals_3[#____self_signals_3 + 1] = sig
                coord = coord + 1
                __continue20 = true
            until true
            if not __continue20 then
                break
            end
        end
    end
    local ____ = coord
end
function World.prototype.buildObstacles(self)
    for ____, b in ipairs(self.city.buildings) do
        do
            local __continue25
            repeat
                if b.kind == "park" or b.kind == "plaza" then
                    __continue25 = true
                    break
                end
                local box = {
                    x = b.x,
                    y = b.y,
                    heading = 0,
                    hl = b.w / 2,
                    hw = b.h / 2
                }
                local ____self_obstacles_4 = self.obstacles
                ____self_obstacles_4[#____self_obstacles_4 + 1] = box
                __continue25 = true
            until true
            if not __continue25 then
                break
            end
        end
    end
    local pad = 16
    local x0 = self.net.minX - pad
    local x1 = self.net.maxX + pad
    local y0 = self.net.minY - pad
    local y1 = self.net.maxY + pad
    local cx = (x0 + x1) / 2
    local cy = (y0 + y1) / 2
    local t = 3
    local ____self_obstacles_5 = self.obstacles
    ____self_obstacles_5[#____self_obstacles_5 + 1] = {
        x = cx,
        y = y0 - t,
        heading = 0,
        hl = (x1 - x0) / 2 + t * 2,
        hw = t
    }
    local ____self_obstacles_6 = self.obstacles
    ____self_obstacles_6[#____self_obstacles_6 + 1] = {
        x = cx,
        y = y1 + t,
        heading = 0,
        hl = (x1 - x0) / 2 + t * 2,
        hw = t
    }
    local ____self_obstacles_7 = self.obstacles
    ____self_obstacles_7[#____self_obstacles_7 + 1] = {
        x = x0 - t,
        y = cy,
        heading = 0,
        hl = t,
        hw = (y1 - y0) / 2 + t * 2
    }
    local ____self_obstacles_8 = self.obstacles
    ____self_obstacles_8[#____self_obstacles_8 + 1] = {
        x = x1 + t,
        y = cy,
        heading = 0,
        hl = t,
        hw = (y1 - y0) / 2 + t * 2
    }
    for ____, o in ipairs(self.obstacles) do
        self.obstacleHash:insert(
            o,
            o.x,
            o.y,
            math.max(o.hl, o.hw)
        )
    end
end
function World.prototype.spawnPlayer(self, cls)
    local best = nil
    local bd = math.huge
    for ____, l in ipairs(self.net.lanes) do
        do
            local __continue31
            repeat
                if l.length < 60 or l.right ~= nil then
                    __continue31 = true
                    break
                end
                local pt = {x = 0, y = 0, heading = 0}
                l.path:sample(l.length * 0.3, pt)
                local d = pt.x * pt.x + pt.y * pt.y
                if d < bd then
                    bd = d
                    best = l
                end
                __continue31 = true
            until true
            if not __continue31 then
                break
            end
        end
    end
    local lane = best or self.net.lanes[1]
    local ____Vehicle_12 = Vehicle
    local ____self_9, ____idCounter_10 = self, "idCounter"
    local ____self_idCounter_11 = ____self_9[____idCounter_10]
    ____self_9[____idCounter_10] = ____self_idCounter_11 + 1
    local v = __TS__New(
        ____Vehicle_12,
        ____self_idCounter_11,
        SPECS[cls],
        16722541,
        "player"
    )
    local pt = {x = 0, y = 0, heading = 0}
    lane.path:sample(lane.length * 0.3, pt)
    v.x = pt.x
    v.y = pt.y
    v.heading = pt.heading
    v.dyn:reset(pt.x, pt.y, pt.heading, 0)
    v.mode = "physics"
    v.lights.headlightSwitch = "auto"
    self.player = v
    self.playerDriver = __TS__New(PlayerDriver, v)
    local ____self_vehicles_13 = self.vehicles
    ____self_vehicles_13[#____self_vehicles_13 + 1] = v
    self.byId:set(v.id, v)
end
function World.prototype.setPlayerVehicle(self, cls)
    local old = self.player
    if old == nil then
        return
    end
    local ____Vehicle_17 = Vehicle
    local ____self_14, ____idCounter_15 = self, "idCounter"
    local ____self_idCounter_16 = ____self_14[____idCounter_15]
    ____self_14[____idCounter_15] = ____self_idCounter_16 + 1
    local v = __TS__New(
        ____Vehicle_17,
        ____self_idCounter_16,
        SPECS[cls],
        old.color,
        "player"
    )
    v.dyn:reset(old.x, old.y, old.heading, 0)
    v.x = old.x
    v.y = old.y
    v.heading = old.heading
    v.mode = "physics"
    v.lights.headlightSwitch = old.lights.headlightSwitch
    self:removeVehicle(old)
    self.player = v
    local pd = __TS__New(PlayerDriver, v)
    if self.playerDriver ~= nil then
        pd.seatbelt = self.playerDriver.seatbelt
        pd.settings.clutchAssist = self.playerDriver.settings.clutchAssist
    end
    self.playerDriver = pd
    local ____self_vehicles_18 = self.vehicles
    ____self_vehicles_18[#____self_vehicles_18 + 1] = v
    self.byId:set(v.id, v)
end
function World.prototype.addVehicle(self, v, d)
    local ____self_vehicles_19 = self.vehicles
    ____self_vehicles_19[#____self_vehicles_19 + 1] = v
    self.byId:set(v.id, v)
    self.drivers:set(v.id, d)
    local lane = v.seg
    d:newDestination(self, lane)
    local ____self_stats_20, ____spawned_21 = self.stats, "spawned"
    ____self_stats_20[____spawned_21] = ____self_stats_20[____spawned_21] + 1
    self.events:emit({type = "spawn", vehicle = v.id})
end
function World.prototype.removeVehicle(self, v)
    local i = __TS__ArrayIndexOf(self.vehicles, v)
    if i >= 0 then
        __TS__ArraySplice(self.vehicles, i, 1)
    end
    self.byId:delete(v.id)
    self.drivers:delete(v.id)
    v.alive = false
    self.allWayQueues:forEach(function(____, q) return q.arrivals:delete(v.id) end)
    self.events:emit({type = "despawn", vehicle = v.id})
end
function World.prototype.laneFree(self, lane, s, clear)
    do
        local i = 0
        while i < lane.occCount do
            if math.abs(lane.occ[i + 1].s - s) < clear then
                return false
            end
            i = i + 1
        end
    end
    for ____, v in ipairs(self.vehicles) do
        if v.seg == lane and math.abs(v.s - s) < clear then
            return false
        end
    end
    return true
end
function World.prototype.vehicle(self, id)
    return self.byId:get(id)
end
function World.prototype.driverOf(self, v)
    return self.drivers:get(v.id)
end
function World.prototype.allWay(self, nodeId)
    local q = self.allWayQueues:get(nodeId)
    if q == nil then
        q = {arrivals = __TS__New(Map)}
        self.allWayQueues:set(nodeId, q)
    end
    return q
end
function World.prototype.policeNear(self, x, y, range)
    local out = {}
    for ____, v in ipairs(self.vehicles) do
        do
            local __continue53
            repeat
                if v.role ~= "police" or not v.alive or v.wrecked then
                    __continue53 = true
                    break
                end
                local dx = v.x - x
                local dy = v.y - y
                if dx * dx + dy * dy > range * range then
                    __continue53 = true
                    break
                end
                if self:los(v.x, v.y, x, y) then
                    out[#out + 1] = v
                end
                __continue53 = true
            until true
            if not __continue53 then
                break
            end
        end
    end
    return out
end
function World.prototype.los(self, ax, ay, bx, by)
    local mx = (ax + bx) / 2
    local my = (ay + by) / 2
    local r = hypot(bx - ax, by - ay) / 2 + 1
    self.obstacleBuf = {}
    self.obstacleHash:query(mx, my, r, self.obstacleBuf)
    for ____, o in ipairs(self.obstacleBuf) do
        local minX = o.x - o.hl + 0.5
        local maxX = o.x + o.hl - 0.5
        local minY = o.y - o.hw + 0.5
        local maxY = o.y + o.hw - 0.5
        local t0 = 0
        local t1 = 1
        local dx = bx - ax
        local dy = by - ay
        local hit = true
        if math.abs(dx) < 1e-9 then
            if ax < minX or ax > maxX then
                hit = false
            end
        else
            local ta = (minX - ax) / dx
            local tb = (maxX - ax) / dx
            if ta > tb then
                local t = ta
                ta = tb
                tb = t
            end
            t0 = math.max(t0, ta)
            t1 = math.min(t1, tb)
            if t0 > t1 then
                hit = false
            end
        end
        if hit then
            if math.abs(dy) < 1e-9 then
                if ay < minY or ay > maxY then
                    hit = false
                end
            else
                local ta = (minY - ay) / dy
                local tb = (maxY - ay) / dy
                if ta > tb then
                    local t = ta
                    ta = tb
                    tb = t
                end
                t0 = math.max(t0, ta)
                t1 = math.min(t1, tb)
                if t0 > t1 then
                    hit = false
                end
            end
        end
        if hit then
            return false
        end
    end
    return true
end
function World.prototype.offend(self, offender, victim, kind, scale)
    if offender == victim then
        return
    end
    local d = self.drivers:get(victim.id)
    if d == nil then
        return
    end
    local gain = d.emo:offend(offender.id, kind, scale, self.time)
    local continuous = kind == "tailgating" or kind == "blocked" or kind == "slow_driver" or kind == "long_wait"
    if not continuous and gain > 0.5 then
        self.events:emit({
            type = "offense",
            offender = offender.id,
            victim = victim.id,
            kind = kind,
            anger = d.emo.anger
        })
        d:onOffended(self, offender, kind)
    end
end
function World.prototype.update(self, realDt)
    self.accumulator = self.accumulator + math.min(realDt, 0.25) * self.timeScale
    local steps = 0
    while self.accumulator >= self.fixedDt and steps < 8 do
        self:step(self.fixedDt)
        self.accumulator = self.accumulator - self.fixedDt
        steps = steps + 1
    end
    if steps == 8 then
        self.accumulator = 0
    end
    return steps
end
function World.prototype.alpha(self)
    return self.accumulator / self.fixedDt
end
function World.prototype.step(self, dt)
    self.time = self.time + dt
    self.env:update(dt)
    self:updateSignals(dt)
    self:rebuildOccupancy()
    self:updatePlayer(dt)
    do
        local i = 0
        while i < #self.vehicles do
            do
                local __continue82
                repeat
                    local v = self.vehicles[i + 1]
                    local d = self.drivers:get(v.id)
                    if d == nil then
                        __continue82 = true
                        break
                    end
                    d:update(dt, self)
                    if d.emo.mood ~= d.lastMood then
                        self.events:emit({type = "mood", vehicle = v.id, mood = d.emo.mood, target = d.emo.target})
                        d.lastMood = d.emo.mood
                    end
                    __continue82 = true
                until true
                if not __continue82 then
                    break
                end
            end
            i = i + 1
        end
    end
    self:physics(dt)
    self:collisions()
    self:lawAndOrder(dt)
    self:ambulances(dt)
    self.spawner:update(dt, self.env.trafficDensity)
    self.cleanupTimer = self.cleanupTimer - dt
    if self.cleanupTimer > 0 then
        return
    end
    self.cleanupTimer = 2
    self.allWayQueues:forEach(function(____, q)
        q.arrivals:forEach(function(____, t, id)
            if self.time - t > 60 or not self.byId:has(id) then
                q.arrivals:delete(id)
            end
        end)
    end)
end
function World.prototype.updateSignals(self, dt)
    local flash = self.env.nightFlash
    if flash ~= self.nightFlashing then
        self.nightFlashing = flash
        for ____, s in ipairs(self.signals) do
            local minor = __TS__ArrayEvery(
                s.node.roads,
                function(____, r) return r.kind ~= "avenue" and r.kind ~= "ring" end
            )
            if minor then
                s:setMode(flash and "flashing" or "actuated")
            end
        end
    end
    for ____, s in ipairs(self.signals) do
        s:update(dt, self.demandFn)
    end
end
function World.prototype.rebuildOccupancy(self)
    self.net:clearOccupancy()
    do
        local i = 0
        while i < #self.vehicles do
            do
                local __continue98
                repeat
                    local v = self.vehicles[i + 1]
                    if not v.alive then
                        __continue98 = true
                        break
                    end
                    if v.mode == "lane" and v.seg ~= nil then
                        local seg = v.seg
                        addOccupant(
                            seg,
                            v,
                            v.s,
                            v.speed,
                            false,
                            v.lat
                        )
                        local prev = v.prevSeg
                        if prev ~= nil and v.s < v.halfLen + 0.5 then
                            addOccupant(
                                prev,
                                v,
                                prev.length + v.s,
                                v.speed,
                                false,
                                v.lat
                            )
                        end
                        local d = self.drivers:get(v.id)
                        if d ~= nil and d.shadowLane ~= nil and seg.kind == "lane" then
                            local sh = d.shadowLane
                            local sS = v.s * sh.length / seg.length
                            local offset = v.lat > 0 and v.lat - (sh.width + seg.width) / 2 or v.lat + (sh.width + seg.width) / 2
                            addOccupant(
                                sh,
                                v,
                                sS,
                                v.speed,
                                true,
                                offset
                            )
                        end
                    else
                        self:registerFreeBody(v)
                    end
                    __continue98 = true
                until true
                if not __continue98 then
                    break
                end
            end
            i = i + 1
        end
    end
end
function World.prototype.registerFreeBody(self, v)
    self.laneHits = {}
    self.net:nearestLanes(v.x, v.y, 7, self.laneHits)
    local ch = math.cos(v.heading)
    local sh = math.sin(v.heading)
    for ____, h in ipairs(self.laneHits) do
        do
            local __continue105
            repeat
                local lh = h.lane.path:headingAt(h.s)
                local rel = math.abs(math.sin(v.heading - lh))
                local halfAcross = v.halfWid * (1 - rel) + v.halfLen * rel
                if math.abs(h.lateral) > h.lane.width / 2 + halfAcross - 0.3 then
                    __continue105 = true
                    break
                end
                if h.s < -v.halfLen or h.s > h.lane.length + v.halfLen then
                    __continue105 = true
                    break
                end
                local vAlong = v:vx() * math.cos(lh) + v:vy() * math.sin(lh)
                addOccupant(
                    h.lane,
                    v,
                    h.s,
                    vAlong,
                    true,
                    h.lateral
                )
                __continue105 = true
            until true
            if not __continue105 then
                break
            end
        end
    end
    self.connHits = {}
    self.net:nearestConnectors(v.x, v.y, 10, self.connHits)
    local pr = {s = 0, lateral = 0, d2 = 0}
    for ____, c in ipairs(self.connHits) do
        do
            local __continue109
            repeat
                c.path:project(v.x, v.y, pr)
                if pr.d2 > 2.6 * 2.6 then
                    __continue109 = true
                    break
                end
                if pr.s <= 0.01 and pr.d2 > 1 then
                    __continue109 = true
                    break
                end
                local lh = c.path:headingAt(pr.s)
                local vAlong = v:vx() * math.cos(lh) + v:vy() * math.sin(lh)
                addOccupant(
                    c,
                    v,
                    pr.s,
                    vAlong,
                    true,
                    pr.lateral
                )
                __continue109 = true
            until true
            if not __continue109 then
                break
            end
        end
    end
    local ____ = ch
    local ____ = sh
end
function World.prototype.updatePlayer(self, dt)
    local p = self.player
    local pd = self.playerDriver
    if p == nil or pd == nil then
        return
    end
    local frame = self.input:frame(dt, p.dyn.u)
    pd:update(dt, frame, self, self.monitor.info.speedLimit)
    for ____, a in ipairs(pd.hostActions) do
        local ____self_hostActions_22 = self.hostActions
        ____self_hostActions_22[#____self_hostActions_22 + 1] = a
    end
    __TS__ArraySplice(pd.hostActions, 0, #pd.hostActions)
    if __TS__ArrayIndexOf(frame.pressed, "reset_vehicle") >= 0 then
        self:resetPlayer()
    end
    if __TS__ArrayIndexOf(frame.pressed, "time_forward") >= 0 then
        self.env.hour = mod(self.env.hour + 1, 24)
    end
    if __TS__ArrayIndexOf(frame.pressed, "weather_cycle") >= 0 then
        local order = {
            "clear",
            "rain",
            "storm",
            "fog",
            "cloudy"
        }
        local i = __TS__ArrayIndexOf(order, self.env.weather)
        self.env:setWeather(order[mod(i + 1, #order) + 1])
    end
    self.monitor.seatbelt = pd.seatbelt
    local lane = self.monitor.info.lane
    p.intentConnector = -1
    if lane ~= nil then
        local want = p.lights.indicator == "left" and "left" or (p.lights.indicator == "right" and "right" or "straight")
        for ____, c in ipairs(lane.outgoing) do
            if c.turn == want then
                p.intentConnector = c.id
            end
        end
    end
    if pd.gesture ~= nil then
        self:gesture(p, pd.gesture)
    end
    if __TS__ArrayIndexOf(frame.pressed, "hazards") >= 0 and p.lights.hazard then
        self:thankYouBlink(p)
    end
    if __TS__ArrayIndexOf(frame.pressed, "horn") >= 0 then
        local ____self_stats_23, ____horns_24 = self.stats, "horns"
        ____self_stats_23[____horns_24] = ____self_stats_23[____horns_24] + 1
        hornVictims(self, p, false)
    end
    for ____, r in ipairs(frame.released) do
        if r.id == "horn" and r.held > 0.9 then
            hornVictims(self, p, true)
        end
    end
    if __TS__ArrayIndexOf(frame.pressed, "flash") >= 0 then
        self:flashAhead(p)
    end
end
function World.prototype.gesture(self, p, kind)
    self.events:emit({type = "gesture", from = p.id, kind = kind})
    for ____, v in ipairs(self.vehicles) do
        do
            local __continue132
            repeat
                local d = self.drivers:get(v.id)
                if d == nil then
                    __continue132 = true
                    break
                end
                local dist = hypot(v.x - p.x, v.y - p.y)
                if kind == "wave" and dist < 45 then
                    if d.emo:grudgeAgainst(p.id) <= 0 then
                        __continue132 = true
                        break
                    end
                    local calmed = d.emo:apologize(p.id, self.time)
                    self.events:emit({type = "apology", from = p.id, to = v.id, calmed = calmed})
                    d.speechCooldown = 0
                    d:say(
                        self,
                        d.emo.anger < 35 and self.rng:pick(APOLOGY_ACCEPTED) or self.rng:pick(APOLOGY_REJECTED)
                    )
                elseif kind == "rude" and dist < 30 then
                    if d.p.archetype == "zen" then
                        d.speechCooldown = 0
                        d:say(self, "Have a nice day!")
                        __continue132 = true
                        break
                    end
                    self:offend(p, v, "rude_gesture", 1)
                end
                __continue132 = true
            until true
            if not __continue132 then
                break
            end
        end
    end
end
function World.prototype.thankYouBlink(self, p)
    for ____, v in ipairs(self.vehicles) do
        do
            local __continue140
            repeat
                local d = self.drivers:get(v.id)
                if d == nil or d.emo:grudgeAgainst(p.id) <= 0 then
                    __continue140 = true
                    break
                end
                local dx = v.x - p.x
                local dy = v.y - p.y
                local behind = dx * math.cos(p.heading) + dy * math.sin(p.heading) < 0
                if not behind or dx * dx + dy * dy > 50 * 50 then
                    __continue140 = true
                    break
                end
                local calmed = d.emo:apologize(p.id, self.time) * 0.6
                self.events:emit({type = "apology", from = p.id, to = v.id, calmed = calmed})
                __continue140 = true
            until true
            if not __continue140 then
                break
            end
        end
    end
end
function World.prototype.flashAhead(self, p)
    for ____, v in ipairs(self.vehicles) do
        do
            local __continue145
            repeat
                if v == p then
                    __continue145 = true
                    break
                end
                local dx = v.x - p.x
                local dy = v.y - p.y
                local fwd = dx * math.cos(p.heading) + dy * math.sin(p.heading)
                if fwd < 0 or fwd > 45 then
                    __continue145 = true
                    break
                end
                local side = math.abs(-dx * math.sin(p.heading) + dy * math.cos(p.heading))
                if side > 3 + fwd * 0.15 then
                    __continue145 = true
                    break
                end
                local last = self.flashMemo:get(v.id) or -10
                if self.time - last < 1.5 then
                    __continue145 = true
                    break
                end
                self.flashMemo:set(v.id, self.time)
                self:offend(p, v, "high_beams", 1)
                __continue145 = true
            until true
            if not __continue145 then
                break
            end
        end
    end
end
function World.prototype.physics(self, dt)
    local grip = self.env.grip
    do
        local i = 0
        while i < #self.vehicles do
            do
                local __continue152
                repeat
                    local v = self.vehicles[i + 1]
                    if v.mode ~= "physics" then
                        __continue152 = true
                        break
                    end
                    local pt = v.pt
                    local dyn = v.dyn
                    local isPlayer = v.isPlayer
                    if v.wrecked then
                        pt.running = false
                        v.inputs.throttle = 0
                    end
                    local throttle = v.inputs.throttle
                    if dyn.tcsActive then
                        throttle = throttle * 0.4
                    end
                    if dyn.escActive then
                        throttle = throttle * 0.6
                    end
                    local torque = pt:update(
                        dt,
                        dyn:drivenOmega(),
                        throttle,
                        dyn.u
                    )
                    local ____isPlayer_25
                    if isPlayer then
                        ____isPlayer_25 = self.monitor.info.onRoad
                    else
                        ____isPlayer_25 = true
                    end
                    local onRoad = ____isPlayer_25
                    dyn.surfaceGrip = grip * (onRoad and 1 or 0.82)
                    dyn.surfaceDrag = onRoad and 0 or 1.5
                    dyn:step(
                        dt,
                        v.inputs,
                        torque,
                        v.steerTarget,
                        v.assists
                    )
                    v:syncFromDynamics()
                    v.wheelSpin = v.wheelSpin + dyn:drivenOmega() * dt
                    v.odometer = v.odometer + dyn:absSpeed() * dt
                    if isPlayer then
                        v.lights:update(dt, dyn.steerAngle)
                        if dyn:absSpeed() > 0.5 then
                            self.record:driveClean(dyn:absSpeed() * dt)
                        end
                    end
                    __continue152 = true
                until true
                if not __continue152 then
                    break
                end
            end
            i = i + 1
        end
    end
    do
        local i = 0
        while i < #self.vehicles do
            do
                local __continue159
                repeat
                    local v = self.vehicles[i + 1]
                    if v.mode ~= "lane" then
                        __continue159 = true
                        break
                    end
                    local spec = v.spec
                    local wheel = v.speed / spec.wheelRadius * spec.finalDrive * 9.549
                    local g = v.gearEstimate
                    if wheel * spec.gears[g] > spec.redline * 0.55 and g < #spec.gears then
                        g = g + 1
                    elseif g > 1 and wheel * spec.gears[g - 2 + 1] < spec.redline * 0.42 then
                        g = g - 1
                    end
                    v.gearEstimate = g
                    v.rpmEstimate = math.max(spec.idleRpm, wheel * spec.gears[g]) + math.max(0, v.accel) * 300
                    __continue159 = true
                until true
                if not __continue159 then
                    break
                end
            end
            i = i + 1
        end
    end
end
function World.prototype.collisions(self)
    self.vehicleHash = __TS__New(SpatialHash, 12)
    for ____, v in ipairs(self.vehicles) do
        if v.alive then
            self.vehicleHash:insert(v, v.x, v.y, v.halfLen)
        end
    end
    do
        local i = 0
        while i < #self.vehicles do
            do
                local __continue167
                repeat
                    local a = self.vehicles[i + 1]
                    if not a.alive then
                        __continue167 = true
                        break
                    end
                    self.queryBuf = {}
                    self.vehicleHash:query(a.x, a.y, a.halfLen + 2, self.queryBuf)
                    for ____, b in ipairs(self.queryBuf) do
                        do
                            local __continue169
                            repeat
                                if b.id <= a.id or not b.alive then
                                    __continue169 = true
                                    break
                                end
                                if a.mode == "lane" and b.mode == "lane" then
                                    if not lanePairCanCollide(a, b) then
                                        __continue169 = true
                                        break
                                    end
                                    local shrink = 0.85
                                    setBox(boxA, a, shrink)
                                    setBox(boxB, b, shrink)
                                else
                                    setBox(boxA, a, 1)
                                    setBox(boxB, b, 1)
                                end
                                if not boxContact(boxA, boxB, contact) then
                                    __continue169 = true
                                    break
                                end
                                self:resolveVehicles(a, b)
                                __continue169 = true
                            until true
                            if not __continue169 then
                                break
                            end
                        end
                    end
                    if a.mode == "physics" then
                        self:staticCollisions(a)
                    end
                    __continue167 = true
                until true
                if not __continue167 then
                    break
                end
            end
            i = i + 1
        end
    end
end
function World.prototype.resolveVehicles(self, a, b)
    local vn = (b:vx() - a:vx()) * contact.nx + (b:vy() - a:vy()) * contact.ny
    local closing = -vn
    local bothLane = a.mode == "lane" and b.mode == "lane"
    if bothLane and closing < 2 then
        return
    end
    if a.mode == "physics" and b.mode == "lane" and closing < 1.2 then
        resolveStatic(a.dyn, contact, 0.1, 0.3)
        a:syncFromDynamics()
        self:bumped(a, b)
        return
    end
    if b.mode == "physics" and a.mode == "lane" and closing < 1.2 then
        local c2 = {
            nx = -contact.nx,
            ny = -contact.ny,
            depth = contact.depth,
            px = contact.px,
            py = contact.py
        }
        resolveStatic(b.dyn, c2, 0.1, 0.3)
        b:syncFromDynamics()
        self:bumped(b, a)
        return
    end
    local aWasLane = a.mode == "lane"
    local bWasLane = b.mode == "lane"
    if aWasLane then
        a:enterPhysics()
    end
    if bWasLane then
        b:enterPhysics()
    end
    local aInto = a:vx() * contact.nx + a:vy() * contact.ny
    local bInto = -(b:vx() * contact.nx + b:vy() * contact.ny)
    local atFault = aInto >= bInto and a or b
    local j = resolveBodies(
        a.dyn,
        b.dyn,
        contact,
        0.15,
        0.4
    )
    a:syncFromDynamics()
    b:syncFromDynamics()
    if j <= 0 then
        return
    end
    local sevA = j / a.dyn.mass
    local sevB = j / b.dyn.mass
    self:damage(a, sevA)
    self:damage(b, sevB)
    local severity = math.max(sevA, sevB)
    local da = self.drivers:get(a.id)
    local db = self.drivers:get(b.id)
    if da ~= nil and (aWasLane or severity > 2) then
        da:onCrash(self, sevA)
    end
    if db ~= nil and (bWasLane or severity > 2) then
        db:onCrash(self, sevB)
    end
    local victim = atFault == a and b or a
    a.lastHitTime = self.time
    b.lastHitTime = self.time
    a.lastHitBy = b.id
    b.lastHitBy = a.id
    if severity > 0.8 then
        local ____self_stats_26, ____collisions_27 = self.stats, "collisions"
        ____self_stats_26[____collisions_27] = ____self_stats_26[____collisions_27] + 1
        self.events:emit({
            type = "collision",
            a = a.id,
            b = b.id,
            impulse = j,
            severity = severity,
            x = contact.px,
            y = contact.py,
            atFault = atFault.id
        })
        self:offend(
            atFault,
            victim,
            "collision",
            clamp(severity / 4, 0.5, 2.5)
        )
        if victim ~= atFault then
            self:offend(victim, atFault, "collision", 0.3)
        end
    end
    local p = self.player
    if p ~= nil and atFault == p and severity > 1.5 then
        self.monitor.hitRun = {
            x = p.x,
            y = p.y,
            time = self.time,
            stoppedFor = 0,
            victim = victim.id
        }
        self:observe({kind = "collision"})
    end
    if severity > 7 then
        self:requestAmbulance(contact.px, contact.py)
    end
end
function World.prototype.bumped(self, phys, laneCar)
    local last = self.nearMissMemo:get(laneCar.id) or -10
    if self.time - last < 2 then
        return
    end
    self.nearMissMemo:set(laneCar.id, self.time)
    self:offend(phys, laneCar, "near_miss", 0.6)
end
function World.prototype.staticCollisions(self, v)
    self.obstacleBuf = {}
    self.obstacleHash:query(v.x, v.y, v.halfLen + 1, self.obstacleBuf)
    for ____, o in ipairs(self.obstacleBuf) do
        do
            local __continue193
            repeat
                setBox(boxA, v, 1)
                if not boxContact(boxA, o, contact) then
                    __continue193 = true
                    break
                end
                local j = resolveStatic(v.dyn, contact, 0.12, 0.5)
                v:syncFromDynamics()
                if j > 0 then
                    local sev = j / v.dyn.mass
                    self:damage(v, sev)
                    if sev > 1.5 then
                        self.events:emit({
                            type = "crash_static",
                            vehicle = v.id,
                            severity = sev,
                            x = contact.px,
                            y = contact.py
                        })
                    end
                    local d = self.drivers:get(v.id)
                    if d ~= nil and sev > 3 and d.state == "chasing" then
                        d:onCrash(self, sev)
                    end
                end
                __continue193 = true
            until true
            if not __continue193 then
                break
            end
        end
    end
end
function World.prototype.damage(self, v, severity)
    local c = math.cos(v.heading)
    local s = math.sin(v.heading)
    local lx = (contact.px - v.x) * c + (contact.py - v.y) * s
    local ly = -(contact.px - v.x) * s + (contact.py - v.y) * c
    local hit = v.damage:hit(
        lx,
        ly,
        v.halfLen,
        v.halfWid,
        severity
    )
    if hit.lost > 0 then
        if hit.zone == "front" then
            v.pt.health = math.min(v.pt.health, 0.25 + v.damage.front * 0.75)
            if v.damage.front < 0.5 then
                if ly > 0 then
                    v.lights.leftHeadlightBroken = true
                else
                    v.lights.rightHeadlightBroken = true
                end
            end
            if v.damage.front < 0.15 then
                v.pt.health = 0
            end
        end
        v.dyn.tireHealth = math.min(
            v.dyn.tireHealth,
            0.5 + v.damage:body() * 0.5
        )
        if v.damage:totaled() and not v.wrecked then
            v.wrecked = true
            v.lights.hazard = not v.isPlayer
            if v.isPlayer then
                self.events:emit({type = "info", text = "Your vehicle is wrecked. Press ` to reset."})
            end
        end
    end
end
function World.prototype.resetPlayer(self)
    local p = self.player
    if p == nil then
        return
    end
    local hits = self.net:nearestLanes(p.x, p.y, 200, {})
    if #hits == 0 then
        return
    end
    local h = hits[1]
    local s = clamp(h.s, 5, h.lane.length - 5)
    local pt = {x = 0, y = 0, heading = 0}
    h.lane.path:sample(s, pt)
    p.dyn:reset(pt.x, pt.y, pt.heading, 0)
    p:syncFromDynamics()
    local cost = math.floor((1 - p.damage:body()) * 1500 + (1 - p.pt.health) * 800)
    if cost > 0 then
        local ____self_record_28, ____money_29 = self.record, "money"
        ____self_record_28[____money_29] = ____self_record_28[____money_29] - cost
        self.events:emit({
            type = "info",
            text = "Vehicle repaired: -€$" .. tostring(cost)
        })
    end
    p.damage:repair()
    p.pt.health = 1
    p.pt.running = true
    p.pt.stalled = false
    p.wrecked = false
    p.dyn.tireHealth = 1
    p.lights.leftHeadlightBroken = false
    p.lights.rightHeadlightBroken = false
    if p.pt.fuel < 5 then
        p.pt.fuel = p.spec.fuelCapacity * 0.5
    end
end
function World.prototype.observe(self, o)
    local ____self_observations_30 = self.observations
    ____self_observations_30[#____self_observations_30 + 1] = o
end
function World.prototype.lawAndOrder(self, dt)
    local p = self.player
    if p ~= nil then
        self.monitor:update(dt, p, self, self.observations)
        for ____, o in ipairs(self.observations) do
            self:handleViolation(p, o.kind, o.cameraNode ~= nil, o.speedCamera == true)
        end
        self.observations = {}
        self:playerSocial(dt, p)
    end
    self.police:update(dt, self, self.record)
end
function World.prototype.handleViolation(self, p, kind, redCamera, speedCamera)
    if not self.record:canCharge(kind, self.time) then
        return
    end
    local cops = self:policeNear(p.x, p.y, 110)
    local witness = "none"
    if #cops > 0 then
        witness = "police"
    elseif redCamera or speedCamera then
        witness = "camera"
    end
    local rec = self.record:add(
        kind,
        self.time,
        witness,
        p.x,
        p.y
    )
    self.events:emit({
        type = "violation",
        vehicle = p.id,
        kind = kind,
        label = rec.label,
        fine = rec.fine,
        points = rec.points,
        witnessed = witness,
        x = p.x,
        y = p.y
    })
    if witness == "camera" then
        self.events:emit({type = "camera_flash", x = p.x, y = p.y, kind = redCamera and "red_light" or "speed"})
        self.events:emit({
            type = "ticket",
            vehicle = p.id,
            total = rec.fine,
            points = rec.points,
            reason = rec.label .. " (camera)"
        })
    elseif witness == "police" then
        self.police:report(self, cops[1], self.record, VIOLATIONS[kind].heat)
    end
end
function World.prototype.playerSocial(self, dt, p)
    local info = self.monitor.info
    local lane = info.lane
    if lane ~= nil and info.headway < 0.7 and p:absSpeed() > 6 and info.leaderGap < 20 then
        local leader = nil
        local best = math.huge
        do
            local i = 0
            while i < lane.occCount do
                do
                    local __continue226
                    repeat
                        local e = lane.occ[i + 1]
                        if e.occ.id == p.id then
                            __continue226 = true
                            break
                        end
                        local v = self.byId:get(e.occ.id)
                        if v == nil then
                            __continue226 = true
                            break
                        end
                        local d = (v.x - p.x) * math.cos(p.heading) + (v.y - p.y) * math.sin(p.heading)
                        if d > 0 and d < best then
                            best = d
                            leader = v
                        end
                        __continue226 = true
                    until true
                    if not __continue226 then
                        break
                    end
                end
                i = i + 1
            end
        end
        if leader ~= nil then
            local t = (self.playerTailgate:get(leader.id) or 0) + dt
            self.playerTailgate:set(leader.id, t)
            if t > 2 then
                self:offend(p, leader, "tailgating", dt * 2.5)
            end
        end
    else
        self.playerTailgate = __TS__New(Map)
    end
    if p:absSpeed() > 8 then
        self.queryBuf = {}
        self.vehicleHash:query(p.x, p.y, 6, self.queryBuf)
        for ____, v in ipairs(self.queryBuf) do
            do
                local __continue234
                repeat
                    if v == p then
                        __continue234 = true
                        break
                    end
                    local d = hypot(v.x - p.x, v.y - p.y) - (v.halfWid + p.halfWid)
                    local rel = hypot(
                        v:vx() - p:vx(),
                        v:vy() - p:vy()
                    )
                    if d < 1.2 and rel > 7 then
                        local last = self.nearMissMemo:get(v.id) or -10
                        if self.time - last < 3 then
                            __continue234 = true
                            break
                        end
                        self.nearMissMemo:set(v.id, self.time)
                        self.events:emit({type = "near_miss", a = p.id, b = v.id})
                        self:offend(
                            p,
                            v,
                            "near_miss",
                            clamp(rel / 12, 0.5, 2)
                        )
                    end
                    __continue234 = true
                until true
                if not __continue234 then
                    break
                end
            end
        end
    end
end
function World.prototype.requestAmbulance(self, x, y)
    if self.ambulance ~= nil and self.ambulance.alive then
        return
    end
    local spot = self.spawner:findSpot(200, 700)
    if spot == nil then
        return
    end
    local v = self.spawner:create(
        "ambulance",
        "ambulance",
        spot.lane,
        spot.s,
        8
    )
    local d = self.drivers:get(v.id)
    if d == nil then
        return
    end
    d.emergency = true
    v.lights.siren = "wail"
    v.lights.lightbar = true
    local hits = self.net:nearestLanes(x, y, 30, {})
    if #hits > 0 then
        d:routeTo(self, hits[1].lane.road, hits[1].lane.forward)
    end
    self.ambulance = v
    self.ambulanceTarget = {x = x, y = y, t = 0}
    self.events:emit({
        type = "dispatch",
        unit = v.id,
        kind = "ambulance",
        x = x,
        y = y,
        reason = "collision"
    })
end
function World.prototype.ambulances(self, dt)
    local a = self.ambulance
    local t = self.ambulanceTarget
    if a == nil or t == nil then
        return
    end
    if not a.alive then
        self.ambulance = nil
        return
    end
    local d = self.drivers:get(a.id)
    local dist = hypot(a.x - t.x, a.y - t.y)
    t.t = t.t + dt
    if (dist < 30 or t.t > 120) and d ~= nil and d.emergency then
        d.emergency = false
        a.lights.siren = "off"
        t.t = 0
    end
    if d ~= nil and not d.emergency and t.t > 30 then
        a.lights.lightbar = false
        self.ambulance = nil
        self.ambulanceTarget = nil
    end
end
function World.prototype.signalAt(self, nodeId)
    local n = self.net.nodes[nodeId + 1]
    return n.signalId >= 0 and self.signals[n.signalId + 1] or nil
end
function World.prototype.vehiclesNear(self, x, y, r)
    local out = {}
    for ____, v in ipairs(self.vehicles) do
        if (v.x - x) * (v.x - x) + (v.y - y) * (v.y - y) <= r * r then
            out[#out + 1] = v
        end
    end
    return out
end
function World.prototype.drainHostActions(self)
    return __TS__ArraySplice(self.hostActions, 0, #self.hostActions)
end
function World.prototype.segment(self, id)
    return self.net.segments[id + 1]
end
function World.prototype.angryAtPlayer(self)
    local out = {}
    local p = self.player
    if p == nil then
        return out
    end
    for ____, v in ipairs(self.vehicles) do
        do
            local __continue258
            repeat
                local d = self.drivers:get(v.id)
                if d == nil then
                    __continue258 = true
                    break
                end
                local g = d.emo:grudgeAgainst(p.id)
                if g > 10 and hypot(v.x - p.x, v.y - p.y) < 120 then
                    out[#out + 1] = {vehicle = v, anger = d.emo.anger, raging = d.rage ~= nil and d.rage.target == p.id}
                end
                __continue258 = true
            until true
            if not __continue258 then
                break
            end
        end
    end
    __TS__ArraySort(
        out,
        function(____, a, b) return b.anger ~= a.anger and b.anger - a.anger or a.vehicle.id - b.vehicle.id end
    )
    return out
end
function World.prototype.isAngleAligned(self, a, b)
    return math.abs(angleDiff(a.heading, b.heading)) < 30 * DEG
end
return ____exports
 end,
["src.core.index"] = function(...) 
local ____lualib = require("lualib_bundle")
local __TS__New = ____lualib.__TS__New
local ____exports = {}
local ____world = require("src.core.world.world")
local WorldClass = ____world.World
do
    local ____world = require("src.core.world.world")
    local World = ____world.World
    ____exports.World = World
end
do
    local ____environment = require("src.core.world.environment")
    local Environment = ____environment.Environment
    ____exports.Environment = Environment
end
do
    local ____cityBuilder = require("src.core.road.cityBuilder")
    local buildCity = ____cityBuilder.buildCity
    ____exports.buildCity = buildCity
end
do
    local ____network = require("src.core.road.network")
    local Network = ____network.Network
    local Lane = ____network.Lane
    local Connector = ____network.Connector
    local Node = ____network.Node
    local Road = ____network.Road
    local Approach = ____network.Approach
    ____exports.Network = Network
    ____exports.Lane = Lane
    ____exports.Connector = Connector
    ____exports.Node = Node
    ____exports.Road = Road
    ____exports.Approach = Approach
end
do
    local ____router = require("src.core.road.router")
    local Router = ____router.Router
    ____exports.Router = Router
end
do
    local ____signalController = require("src.core.signals.signalController")
    local SignalController = ____signalController.SignalController
    ____exports.SignalController = SignalController
end
do
    local ____vehicle = require("src.core.vehicle.vehicle")
    local Vehicle = ____vehicle.Vehicle
    ____exports.Vehicle = Vehicle
end
do
    local ____dynamics = require("src.core.vehicle.dynamics")
    local VehicleDynamics = ____dynamics.VehicleDynamics
    ____exports.VehicleDynamics = VehicleDynamics
end
do
    local ____powertrain = require("src.core.vehicle.powertrain")
    local Powertrain = ____powertrain.Powertrain
    ____exports.Powertrain = Powertrain
end
do
    local ____lights = require("src.core.vehicle.lights")
    local VehicleLights = ____lights.VehicleLights
    ____exports.VehicleLights = VehicleLights
end
do
    local ____specs = require("src.core.vehicle.specs")
    local SPECS = ____specs.SPECS
    local CIVILIAN_CLASSES = ____specs.CIVILIAN_CLASSES
    local topSpeed = ____specs.topSpeed
    local maxAccelAt = ____specs.maxAccelAt
    ____exports.SPECS = SPECS
    ____exports.CIVILIAN_CLASSES = CIVILIAN_CLASSES
    ____exports.topSpeed = topSpeed
    ____exports.maxAccelAt = maxAccelAt
end
do
    local ____driver = require("src.core.ai.driver")
    local AIDriver = ____driver.AIDriver
    ____exports.AIDriver = AIDriver
end
do
    local ____emotion = require("src.core.ai.emotion")
    local Emotion = ____emotion.Emotion
    local OFFENSE_ANGER = ____emotion.OFFENSE_ANGER
    ____exports.Emotion = Emotion
    ____exports.OFFENSE_ANGER = OFFENSE_ANGER
end
do
    local ____personality = require("src.core.ai.personality")
    local samplePersonality = ____personality.samplePersonality
    local ARCHETYPES = ____personality.ARCHETYPES
    ____exports.samplePersonality = samplePersonality
    ____exports.ARCHETYPES = ARCHETYPES
end
do
    local ____idm = require("src.core.ai.idm")
    local idmAccel = ____idm.idmAccel
    local idmFree = ____idm.idmFree
    ____exports.idmAccel = idmAccel
    ____exports.idmFree = idmFree
end
do
    local ____actions = require("src.core.player.actions")
    local ACTIONS = ____actions.ACTIONS
    local InputMapper = ____actions.InputMapper
    local actionDef = ____actions.actionDef
    ____exports.ACTIONS = ACTIONS
    ____exports.InputMapper = InputMapper
    ____exports.actionDef = actionDef
end
do
    local ____playerDriver = require("src.core.player.playerDriver")
    local PlayerDriver = ____playerDriver.PlayerDriver
    ____exports.PlayerDriver = PlayerDriver
end
do
    local ____violations = require("src.core.law.violations")
    local DrivingRecord = ____violations.DrivingRecord
    local VIOLATIONS = ____violations.VIOLATIONS
    ____exports.DrivingRecord = DrivingRecord
    ____exports.VIOLATIONS = VIOLATIONS
end
do
    local ____playerMonitor = require("src.core.law.playerMonitor")
    local PlayerMonitor = ____playerMonitor.PlayerMonitor
    ____exports.PlayerMonitor = PlayerMonitor
end
do
    local ____police = require("src.core.law.police")
    local PoliceSystem = ____police.PoliceSystem
    ____exports.PoliceSystem = PoliceSystem
end
do
    local ____rng = require("src.core.math.rng")
    local Rng = ____rng.Rng
    ____exports.Rng = Rng
end
do
    local ____events = require("src.core.util.events")
    local EventBus = ____events.EventBus
    ____exports.EventBus = EventBus
end
____exports.VERSION = "0.1.0"
--- Factory for hosts that cannot use `new` (plain Lua: core.createWorld({ seed = 1 })).
function ____exports.createWorld(opts)
    return __TS__New(WorldClass, opts or ({}))
end
return ____exports
 end,
}
return require("src.core.index", ...)
