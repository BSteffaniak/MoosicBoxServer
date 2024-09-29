const u={context:void 0,registry:void 0,effects:void 0,done:!1,getContextId(){return z(this.context.count)},getNextContextId(){return z(this.context.count++)}};function z(e){const t=String(e),n=t.length-1;return u.context.id+(n?String.fromCharCode(96+n):"")+t}function T(e){u.context=e}function be(){return{...u.context,id:u.getNextContextId(),count:0}}const we=(e,t)=>e===t,Ue=Symbol("solid-proxy"),ie=Symbol("solid-track"),B={equals:we};let re=ae;const k=1,M=2,le={owned:null,cleanups:null,context:null,owner:null};var h=null;let W=null,xe=null,g=null,p=null,m=null,V=0;function I(e,t){const n=g,s=h,i=e.length===0,r=t===void 0?s:t,f=i?le:{owned:null,cleanups:null,context:r?r.context:null,owner:r},o=i?e:()=>e(()=>S(()=>K(f)));h=f,g=null;try{return P(o,!0)}finally{g=n,h=s}}function Y(e,t){t=t?Object.assign({},B,t):B;const n={value:e,observers:null,observerSlots:null,comparator:t.equals||void 0},s=i=>(typeof i=="function"&&(i=i(n.value)),ce(n,i));return[ue.bind(n),s]}function qe(e,t,n){const s=G(e,t,!0,k);F(s)}function j(e,t,n){const s=G(e,t,!1,k);F(s)}function Ae(e,t,n){re=ke;const s=G(e,t,!1,k),i=O&&fe(O);i&&(s.suspense=i),s.user=!0,m?m.push(s):F(s)}function E(e,t,n){n=n?Object.assign({},B,n):B;const s=G(e,t,!0,0);return s.observers=null,s.observerSlots=null,s.comparator=n.equals||void 0,F(s),ue.bind(s)}function De(e){return P(e,!1)}function S(e){if(g===null)return e();const t=g;g=null;try{return e()}finally{g=t}}function Re(e,t,n){const s=Array.isArray(e);let i;return r=>{let f;if(s){f=Array(e.length);for(let l=0;l<e.length;l++)f[l]=e[l]()}else f=e();const o=S(()=>t(f,i,r));return i=f,o}}function Ve(e){Ae(()=>S(e))}function Z(e){return h===null||(h.cleanups===null?h.cleanups=[e]:h.cleanups.push(e)),e}function Ge(){return g}function Ce(){return h}function Se(e){m.push.apply(m,e),e.length=0}function oe(e,t){const n=Symbol("context");return{id:n,Provider:Te(n),defaultValue:e}}function fe(e){let t;return h&&h.context&&(t=h.context[e.id])!==void 0?t:e.defaultValue}function me(e){const t=E(e),n=E(()=>X(t()));return n.toArray=()=>{const s=n();return Array.isArray(s)?s:s!=null?[s]:[]},n}let O;function ve(){return O||(O=oe())}function ue(){if(this.sources&&this.state)if(this.state===k)F(this);else{const e=p;p=null,P(()=>q(this),!1),p=e}if(g){const e=this.observers?this.observers.length:0;g.sources?(g.sources.push(this),g.sourceSlots.push(e)):(g.sources=[this],g.sourceSlots=[e]),this.observers?(this.observers.push(g),this.observerSlots.push(g.sources.length-1)):(this.observers=[g],this.observerSlots=[g.sources.length-1])}return this.value}function ce(e,t,n){let s=e.value;return(!e.comparator||!e.comparator(s,t))&&(e.value=t,e.observers&&e.observers.length&&P(()=>{for(let i=0;i<e.observers.length;i+=1){const r=e.observers[i],f=W&&W.running;f&&W.disposed.has(r),(f?!r.tState:!r.state)&&(r.pure?p.push(r):m.push(r),r.observers&&de(r)),f||(r.state=k)}if(p.length>1e6)throw p=[],new Error},!1)),t}function F(e){if(!e.fn)return;K(e);const t=V;$e(e,e.value,t)}function $e(e,t,n){let s;const i=h,r=g;g=h=e;try{s=e.fn(t)}catch(f){return e.pure&&(e.state=k,e.owned&&e.owned.forEach(K),e.owned=null),e.updatedAt=n+1,he(f)}finally{g=r,h=i}(!e.updatedAt||e.updatedAt<=n)&&(e.updatedAt!=null&&"observers"in e?ce(e,s):e.value=s,e.updatedAt=n)}function G(e,t,n,s=k,i){const r={fn:e,state:s,updatedAt:null,owned:null,sources:null,sourceSlots:null,cleanups:null,value:t,owner:h,context:h?h.context:null,pure:n};return h===null||h!==le&&(h.owned?h.owned.push(r):h.owned=[r]),r}function U(e){if(e.state===0)return;if(e.state===M)return q(e);if(e.suspense&&S(e.suspense.inFallback))return e.suspense.effects.push(e);const t=[e];for(;(e=e.owner)&&(!e.updatedAt||e.updatedAt<V);)e.state&&t.push(e);for(let n=t.length-1;n>=0;n--)if(e=t[n],e.state===k)F(e);else if(e.state===M){const s=p;p=null,P(()=>q(e,t[0]),!1),p=s}}function P(e,t){if(p)return e();let n=!1;t||(p=[]),m?n=!0:m=[],V++;try{const s=e();return Ee(n),s}catch(s){n||(m=null),p=null,he(s)}}function Ee(e){if(p&&(ae(p),p=null),e)return;const t=m;m=null,t.length&&P(()=>re(t),!1)}function ae(e){for(let t=0;t<e.length;t++)U(e[t])}function ke(e){let t,n=0;for(t=0;t<e.length;t++){const s=e[t];s.user?e[n++]=s:U(s)}if(u.context){if(u.count){u.effects||(u.effects=[]),u.effects.push(...e.slice(0,n));return}T()}for(u.effects&&(u.done||!u.count)&&(e=[...u.effects,...e],n+=u.effects.length,delete u.effects),t=0;t<n;t++)U(e[t])}function q(e,t){e.state=0;for(let n=0;n<e.sources.length;n+=1){const s=e.sources[n];if(s.sources){const i=s.state;i===k?s!==t&&(!s.updatedAt||s.updatedAt<V)&&U(s):i===M&&q(s,t)}}}function de(e){for(let t=0;t<e.observers.length;t+=1){const n=e.observers[t];n.state||(n.state=M,n.pure?p.push(n):m.push(n),n.observers&&de(n))}}function K(e){let t;if(e.sources)for(;e.sources.length;){const n=e.sources.pop(),s=e.sourceSlots.pop(),i=n.observers;if(i&&i.length){const r=i.pop(),f=n.observerSlots.pop();s<i.length&&(r.sourceSlots[f]=s,i[s]=r,n.observerSlots[s]=f)}}if(e.owned){for(t=e.owned.length-1;t>=0;t--)K(e.owned[t]);e.owned=null}if(e.cleanups){for(t=e.cleanups.length-1;t>=0;t--)e.cleanups[t]();e.cleanups=null}e.state=0}function Ne(e){return e instanceof Error?e:new Error(typeof e=="string"?e:"Unknown error",{cause:e})}function he(e,t=h){throw Ne(e)}function X(e){if(typeof e=="function"&&!e.length)return X(e());if(Array.isArray(e)){const t=[];for(let n=0;n<e.length;n++){const s=X(e[n]);Array.isArray(s)?t.push.apply(t,s):t.push(s)}return t}return e}function Te(e,t){return function(s){let i;return j(()=>i=S(()=>(h.context={...h.context,[e]:s.value},me(()=>s.children))),void 0),i}}const Q=Symbol("fallback");function D(e){for(let t=0;t<e.length;t++)e[t]()}function Ie(e,t,n={}){let s=[],i=[],r=[],f=0,o=t.length>1?[]:null;return Z(()=>D(r)),()=>{let l=e()||[],d=l.length,a,c;return l[ie],S(()=>{let y,b,w,N,v,x,C,$,H;if(d===0)f!==0&&(D(r),r=[],s=[],i=[],f=0,o&&(o=[])),n.fallback&&(s=[Q],i[0]=I(pe=>(r[0]=pe,n.fallback())),f=1);else if(f===0){for(i=new Array(d),c=0;c<d;c++)s[c]=l[c],i[c]=I(A);f=d}else{for(w=new Array(d),N=new Array(d),o&&(v=new Array(d)),x=0,C=Math.min(f,d);x<C&&s[x]===l[x];x++);for(C=f-1,$=d-1;C>=x&&$>=x&&s[C]===l[$];C--,$--)w[$]=i[C],N[$]=r[C],o&&(v[$]=o[C]);for(y=new Map,b=new Array($+1),c=$;c>=x;c--)H=l[c],a=y.get(H),b[c]=a===void 0?-1:a,y.set(H,c);for(a=x;a<=C;a++)H=s[a],c=y.get(H),c!==void 0&&c!==-1?(w[c]=i[a],N[c]=r[a],o&&(v[c]=o[a]),c=b[c],y.set(H,c)):r[a]();for(c=x;c<d;c++)c in w?(i[c]=w[c],r[c]=N[c],o&&(o[c]=v[c],o[c](c))):i[c]=I(A);i=i.slice(0,f=d),s=l.slice(0)}return i});function A(y){if(r[c]=y,o){const[b,w]=Y(c);return o[c]=w,t(l[c],b)}return t(l[c])}}}function He(e,t,n={}){let s=[],i=[],r=[],f=[],o=0,l;return Z(()=>D(r)),()=>{const d=e()||[],a=d.length;return d[ie],S(()=>{if(a===0)return o!==0&&(D(r),r=[],s=[],i=[],o=0,f=[]),n.fallback&&(s=[Q],i[0]=I(A=>(r[0]=A,n.fallback())),o=1),i;for(s[0]===Q&&(r[0](),r=[],s=[],i=[],o=0),l=0;l<a;l++)l<s.length&&s[l]!==d[l]?f[l](()=>d[l]):l>=s.length&&(i[l]=I(c));for(;l<s.length;l++)r[l]();return o=f.length=r.length=a,s=d.slice(0),i=i.slice(0,o)});function c(A){r[l]=A;const[y,b]=Y(d[l]);return f[l]=b,t(y,l)}}}let ge=!1;function Le(){ge=!0}function Fe(e,t){if(ge&&u.context){const n=u.context;T(be());const s=S(()=>e(t||{}));return T(n),s}return S(()=>e(t||{}))}let Pe=0;function Ke(){return u.context?u.getNextContextId():`cl-${Pe++}`}const _e=e=>`Stale read from <${e}>.`;function We(e){const t="fallback"in e&&{fallback:()=>e.fallback};return E(Ie(()=>e.each,e.children,t||void 0))}function Xe(e){const t="fallback"in e&&{fallback:()=>e.fallback};return E(He(()=>e.each,e.children,t||void 0))}function Qe(e){const t=e.keyed,n=E(()=>e.when,void 0,{equals:(s,i)=>t?s===i:!s==!i});return E(()=>{const s=n();if(s){const i=e.children;return typeof i=="function"&&i.length>0?S(()=>i(t?s:()=>{if(!S(n))throw _e("Show");return e.when})):i}return e.fallback},void 0,void 0)}const Be=oe();function Je(e){let t=0,n,s,i,r,f;const[o,l]=Y(!1),d=ve(),a={increment:()=>{++t===1&&l(!0)},decrement:()=>{--t===0&&l(!1)},inFallback:o,effects:[],resolved:!1},c=Ce();if(u.context&&u.load){const b=u.getContextId();let w=u.load(b);if(w&&(typeof w!="object"||w.status!=="success"?i=w:u.gather(b)),i&&i!=="$$f"){const[N,v]=Y(void 0,{equals:!1});r=N,i.then(()=>{if(u.done)return v();u.gather(b),T(s),v(),T()},x=>{f=x,v()})}}const A=fe(Be);A&&(n=A.register(a.inFallback));let y;return Z(()=>y&&y()),Fe(d.Provider,{value:a,get children(){return E(()=>{if(f)throw f;if(s=u.context,r)return r(),r=void 0;s&&i==="$$f"&&T();const b=E(()=>e.children);return E(w=>{const N=a.inFallback(),{showContent:v=!0,showFallback:x=!0}=n?n():{};if((!N||i&&i!=="$$f")&&v)return a.resolved=!0,y&&y(),y=s=i=void 0,Se(a.effects),b();if(x)return y?w:I(C=>(y=C,s&&(T({id:s.id+"F",count:0}),s=void 0),e.fallback),c)})})}})}function Me(e,t,n){let s=n.length,i=t.length,r=s,f=0,o=0,l=t[i-1].nextSibling,d=null;for(;f<i||o<r;){if(t[f]===n[o]){f++,o++;continue}for(;t[i-1]===n[r-1];)i--,r--;if(i===f){const a=r<s?o?n[o-1].nextSibling:n[r-o]:l;for(;o<r;)e.insertBefore(n[o++],a)}else if(r===o)for(;f<i;)(!d||!d.has(t[f]))&&t[f].remove(),f++;else if(t[f]===n[r-1]&&n[o]===t[i-1]){const a=t[--i].nextSibling;e.insertBefore(n[o++],t[f++].nextSibling),e.insertBefore(n[--r],a),t[i]=n[r]}else{if(!d){d=new Map;let c=o;for(;c<r;)d.set(n[c],c++)}const a=d.get(t[f]);if(a!=null)if(o<a&&a<r){let c=f,A=1,y;for(;++c<i&&c<r&&!((y=d.get(t[c]))==null||y!==a+A);)A++;if(A>a-o){const b=t[f];for(;o<a;)e.insertBefore(n[o++],b)}else e.replaceChild(n[o++],t[f++])}else f++;else t[f++].remove()}}}const ee="_$DX_DELEGATE";function te(e,t,n,s={}){let i;return I(r=>{i=r,t===document?e():Ye(t,e(),t.firstChild?null:void 0,n)},s.owner),()=>{i(),t.textContent=""}}function Ze(e,t,n){let s;const i=()=>{const f=document.createElement("template");return f.innerHTML=e,f.content.firstChild},r=()=>(s||(s=i())).cloneNode(!0);return r.cloneNode=r,r}function ze(e,t=window.document){const n=t[ee]||(t[ee]=new Set);for(let s=0,i=e.length;s<i;s++){const r=e[s];n.has(r)||(n.add(r),t.addEventListener(r,ye))}}function et(e,t,n){_(e)||(e[t]=n)}function tt(e,t,n){_(e)||(n==null?e.removeAttribute(t):e.setAttribute(t,n))}function nt(e,t){_(e)||(t==null?e.removeAttribute("class"):e.className=t)}function st(e,t,n,s){Array.isArray(n)?(e[`$$${t}`]=n[0],e[`$$${t}Data`]=n[1]):e[`$$${t}`]=n}function it(e,t,n){return S(()=>e(t,n))}function Ye(e,t,n,s){if(n!==void 0&&!s&&(s=[]),typeof t!="function")return R(e,t,s,n);j(i=>R(e,t(),i,n),s)}function je(e,t,n={}){if(globalThis._$HY.done)return te(e,t,[...t.childNodes],n);u.completed=globalThis._$HY.completed,u.events=globalThis._$HY.events,u.load=s=>globalThis._$HY.r[s],u.has=s=>s in globalThis._$HY.r,u.gather=s=>se(t,s),u.registry=new Map,u.context={id:n.renderId||"",count:0};try{return se(t,n.renderId),te(e,t,[...t.childNodes],n)}finally{u.context=null}}function rt(e){let t,n;return!_()||!(t=u.registry.get(n=Oe()))?e():(u.completed&&u.completed.add(t),u.registry.delete(n),t)}function lt(e){let t=e,n=0,s=[];if(_(e))for(;t;){if(t.nodeType===8){const i=t.nodeValue;if(i==="$")n++;else if(i==="/"){if(n===0)return[t,s];n--}}s.push(t),t=t.nextSibling}return[t,s]}function ot(){u.events&&!u.events.queued&&(queueMicrotask(()=>{const{completed:e,events:t}=u;for(t.queued=!1;t.length;){const[n,s]=t[0];if(!e.has(n))return;t.shift(),ye(s)}u.done&&(u.events=_$HY.events=null,u.completed=_$HY.completed=null)}),u.events.queued=!0)}function _(e){return!!u.context&&!u.done&&(!e||e.isConnected)}function ye(e){if(u.registry&&u.events&&u.events.find(([s,i])=>i===e))return;const t=`$$${e.type}`;let n=e.composedPath&&e.composedPath()[0]||e.target;for(e.target!==n&&Object.defineProperty(e,"target",{configurable:!0,value:n}),Object.defineProperty(e,"currentTarget",{configurable:!0,get(){return n||document}}),u.registry&&!u.done&&(u.done=_$HY.done=!0);n;){const s=n[t];if(s&&!n.disabled){const i=n[`${t}Data`];if(i!==void 0?s.call(n,i,e):s.call(n,e),e.cancelBubble)return}n=n._$host||n.parentNode||n.host}}function R(e,t,n,s,i){const r=_(e);if(r){!n&&(n=[...e.childNodes]);let l=[];for(let d=0;d<n.length;d++){const a=n[d];a.nodeType===8&&a.data.slice(0,2)==="!$"?a.remove():l.push(a)}n=l}for(;typeof n=="function";)n=n();if(t===n)return n;const f=typeof t,o=s!==void 0;if(e=o&&n[0]&&n[0].parentNode||e,f==="string"||f==="number"){if(r||f==="number"&&(t=t.toString(),t===n))return n;if(o){let l=n[0];l&&l.nodeType===3?l.data!==t&&(l.data=t):l=document.createTextNode(t),n=L(e,n,s,l)}else n!==""&&typeof n=="string"?n=e.firstChild.data=t:n=e.textContent=t}else if(t==null||f==="boolean"){if(r)return n;n=L(e,n,s)}else{if(f==="function")return j(()=>{let l=t();for(;typeof l=="function";)l=l();n=R(e,l,n,s)}),()=>n;if(Array.isArray(t)){const l=[],d=n&&Array.isArray(n);if(J(l,t,n,i))return j(()=>n=R(e,l,n,s,!0)),()=>n;if(r){if(!l.length)return n;if(s===void 0)return n=[...e.childNodes];let a=l[0];if(a.parentNode!==e)return n;const c=[a];for(;(a=a.nextSibling)!==s;)c.push(a);return n=c}if(l.length===0){if(n=L(e,n,s),o)return n}else d?n.length===0?ne(e,l,s):Me(e,n,l):(n&&L(e),ne(e,l));n=l}else if(t.nodeType){if(r&&t.parentNode)return n=o?[t]:t;if(Array.isArray(n)){if(o)return n=L(e,n,s,t);L(e,n,null,t)}else n==null||n===""||!e.firstChild?e.appendChild(t):e.replaceChild(t,e.firstChild);n=t}}return n}function J(e,t,n,s){let i=!1;for(let r=0,f=t.length;r<f;r++){let o=t[r],l=n&&n[e.length],d;if(!(o==null||o===!0||o===!1))if((d=typeof o)=="object"&&o.nodeType)e.push(o);else if(Array.isArray(o))i=J(e,o,l)||i;else if(d==="function")if(s){for(;typeof o=="function";)o=o();i=J(e,Array.isArray(o)?o:[o],Array.isArray(l)?l:[l])||i}else e.push(o),i=!0;else{const a=String(o);l&&l.nodeType===3&&l.data===a?e.push(l):e.push(document.createTextNode(a))}}return i}function ne(e,t,n=null){for(let s=0,i=t.length;s<i;s++)e.insertBefore(t[s],n)}function L(e,t,n,s){if(n===void 0)return e.textContent="";const i=s||document.createTextNode("");if(t.length){let r=!1;for(let f=t.length-1;f>=0;f--){const o=t[f];if(i!==o){const l=o.parentNode===e;!r&&!f?l?e.replaceChild(i,o):e.insertBefore(i,n):l&&o.remove()}else r=!0}}else e.insertBefore(i,n);return[i]}function se(e,t){const n=e.querySelectorAll("*[data-hk]");for(let s=0;s<n.length;s++){const i=n[s],r=i.getAttribute("data-hk");(!t||r.startsWith(t))&&!u.registry.has(r)&&u.registry.set(r,i)}}function Oe(){return u.getNextContextId()}const ft=(...e)=>(Le(),je(...e));export{Ue as $,De as A,Ke as B,S as C,We as F,Xe as I,Qe as S,lt as a,Fe as b,Y as c,ze as d,E as e,j as f,rt as g,qe as h,Ye as i,Ae as j,nt as k,Ve as l,Ce as m,Z as n,Re as o,ft as p,te as q,ot as r,et as s,Ze as t,it as u,Je as v,tt as w,st as x,ie as y,Ge as z};
//# sourceMappingURL=web.o2PnP_jd.js.map