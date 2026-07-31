import{a as B}from"./vendor-query-CVdC1PZG.js";function Ce(e){var t,o,r="";if(typeof e=="string"||typeof e=="number")r+=e;else if(typeof e=="object")if(Array.isArray(e)){var i=e.length;for(t=0;t<i;t++)e[t]&&(o=Ce(e[t]))&&(r&&(r+=" "),r+=o)}else for(o in e)e[o]&&(r&&(r+=" "),r+=o);return r}function Ee(){for(var e,t,o=0,r="",i=arguments.length;o<i;o++)(e=arguments[o])&&(t=Ce(e))&&(r&&(r+=" "),r+=t);return r}const We=(e,t)=>{const o=new Array(e.length+t.length);for(let r=0;r<e.length;r++)o[r]=e[r];for(let r=0;r<t.length;r++)o[e.length+r]=t[r];return o},Fe=(e,t)=>({classGroupId:e,validator:t}),Ne=(e=new Map,t=null,o)=>({nextPart:e,validators:t,classGroupId:o}),Q="-",ke=[],Be="arbitrary..",Ue=e=>{const t=De(e),{conflictingClassGroups:o,conflictingClassGroupModifiers:r}=e;return{getClassGroupId:d=>{if(d.startsWith("[")&&d.endsWith("]"))return Ze(d);const m=d.split(Q),y=m[0]===""&&m.length>1?1:0;return Ae(m,y,t)},getConflictingClassGroupIds:(d,m)=>{if(m){const y=r[d],p=o[d];return y?p?We(p,y):y:p||ke}return o[d]||ke}}},Ae=(e,t,o)=>{if(e.length-t===0)return o.classGroupId;const i=e[t],l=o.nextPart.get(i);if(l){const p=Ae(e,t+1,l);if(p)return p}const d=o.validators;if(d===null)return;const m=t===0?e.join(Q):e.slice(t).join(Q),y=d.length;for(let p=0;p<y;p++){const b=d[p];if(b.validator(m))return b.classGroupId}},Ze=e=>e.slice(1,-1).indexOf(":")===-1?void 0:(()=>{const t=e.slice(1,-1),o=t.indexOf(":"),r=t.slice(0,o);return r?Be+r:void 0})(),De=e=>{const{theme:t,classGroups:o}=e;return Ye(o,t)},Ye=(e,t)=>{const o=Ne();for(const r in e){const i=e[r];ie(i,o,r,t)}return o},ie=(e,t,o,r)=>{const i=e.length;for(let l=0;l<i;l++){const d=e[l];Ke(d,t,o,r)}},Ke=(e,t,o,r)=>{if(typeof e=="string"){Xe(e,t,o);return}if(typeof e=="function"){Je(e,t,o,r);return}Qe(e,t,o,r)},Xe=(e,t,o)=>{const r=e===""?t:$e(t,e);r.classGroupId=o},Je=(e,t,o,r)=>{if(eo(e)){ie(e(r),t,o,r);return}t.validators===null&&(t.validators=[]),t.validators.push(Fe(o,e))},Qe=(e,t,o,r)=>{const i=Object.entries(e),l=i.length;for(let d=0;d<l;d++){const[m,y]=i[d];ie(y,$e(t,m),o,r)}},$e=(e,t)=>{let o=e;const r=t.split(Q),i=r.length;for(let l=0;l<i;l++){const d=r[l];let m=o.nextPart.get(d);m||(m=Ne(),o.nextPart.set(d,m)),o=m}return o},eo=e=>"isThemeGetter"in e&&e.isThemeGetter===!0,oo=e=>{if(e<1)return{get:()=>{},set:()=>{}};let t=0,o=Object.create(null),r=Object.create(null);const i=(l,d)=>{o[l]=d,t++,t>e&&(t=0,r=o,o=Object.create(null))};return{get(l){let d=o[l];if(d!==void 0)return d;if((d=r[l])!==void 0)return i(l,d),d},set(l,d){l in o?o[l]=d:i(l,d)}}},ne="!",fe=":",to=[],ge=(e,t,o,r,i)=>({modifiers:e,hasImportantModifier:t,baseClassName:o,maybePostfixModifierPosition:r,isExternal:i}),ro=e=>{const{prefix:t,experimentalParseClassName:o}=e;let r=i=>{const l=[];let d=0,m=0,y=0,p;const b=i.length;for(let x=0;x<b;x++){const v=i[x];if(d===0&&m===0){if(v===fe){l.push(i.slice(y,x)),y=x+1;continue}if(v==="/"){p=x;continue}}v==="["?d++:v==="]"?d--:v==="("?m++:v===")"&&m--}const k=l.length===0?i:i.slice(y);let _=k,N=!1;k.endsWith(ne)?(_=k.slice(0,-1),N=!0):k.startsWith(ne)&&(_=k.slice(1),N=!0);const L=p&&p>y?p-y:void 0;return ge(l,N,_,L)};if(t){const i=t+fe,l=r;r=d=>d.startsWith(i)?l(d.slice(i.length)):ge(to,!1,d,void 0,!0)}if(o){const i=r;r=l=>o({className:l,parseClassName:i})}return r},ao=e=>{const t=new Map;return e.orderSensitiveModifiers.forEach((o,r)=>{t.set(o,1e6+r)}),o=>{const r=[];let i=[];for(let l=0;l<o.length;l++){const d=o[l],m=d[0]==="[",y=t.has(d);m||y?(i.length>0&&(i.sort(),r.push(...i),i=[]),r.push(d)):i.push(d)}return i.length>0&&(i.sort(),r.push(...i)),r}},so=e=>({cache:oo(e.cacheSize),parseClassName:ro(e),sortModifiers:ao(e),postfixLookupClassGroupIds:no(e),...Ue(e)}),no=e=>{const t=Object.create(null),o=e.postfixLookupClassGroups;if(o)for(let r=0;r<o.length;r++)t[o[r]]=!0;return t},io=/\s+/,lo=(e,t)=>{const{parseClassName:o,getClassGroupId:r,getConflictingClassGroupIds:i,sortModifiers:l,postfixLookupClassGroupIds:d}=t,m=[],y=e.trim().split(io);let p="";for(let b=y.length-1;b>=0;b-=1){const k=y[b],{isExternal:_,modifiers:N,hasImportantModifier:L,baseClassName:x,maybePostfixModifierPosition:v}=o(k);if(_){p=k+(p.length>0?" "+p:p);continue}let R=!!v,z;if(R){const j=x.substring(0,v);z=r(j);const c=z&&d[z]?r(x):void 0;c&&c!==z&&(z=c,R=!1)}else z=r(x);if(!z){if(!R){p=k+(p.length>0?" "+p:p);continue}if(z=r(x),!z){p=k+(p.length>0?" "+p:p);continue}R=!1}const W=N.length===0?"":N.length===1?N[0]:l(N).join(":"),H=L?W+ne:W,T=H+z;if(m.indexOf(T)>-1)continue;m.push(T);const O=i(z,R);for(let j=0;j<O.length;++j){const c=O[j];m.push(H+c)}p=k+(p.length>0?" "+p:p)}return p},co=(...e)=>{let t=0,o,r,i="";for(;t<e.length;)(o=e[t++])&&(r=je(o))&&(i&&(i+=" "),i+=r);return i},je=e=>{if(typeof e=="string")return e;let t,o="";for(let r=0;r<e.length;r++)e[r]&&(t=je(e[r]))&&(o&&(o+=" "),o+=t);return o},po=(e,...t)=>{let o,r,i,l;const d=y=>{const p=t.reduce((b,k)=>k(b),e());return o=so(p),r=o.cache.get,i=o.cache.set,l=m,m(y)},m=y=>{const p=r(y);if(p)return p;const b=lo(y,o);return i(y,b),b};return l=d,(...y)=>l(co(...y))},ho=[],f=e=>{const t=o=>o[e]||ho;return t.isThemeGetter=!0,t},Se=/^\[(?:(\w[\w-]*):)?(.+)\]$/i,Ve=/^\((?:(\w[\w-]*):)?(.+)\)$/i,mo=/^\d+(?:\.\d+)?\/\d+(?:\.\d+)?$/,uo=/^(\d+(\.\d+)?)?(xs|sm|md|lg|xl)$/,yo=/\d+(%|px|r?em|[sdl]?v([hwib]|min|max)|pt|pc|in|cm|mm|cap|ch|ex|r?lh|cq(w|h|i|b|min|max))|\b(calc|min|max|clamp)\(.+\)|^0$/,bo=/^(rgba?|hsla?|hwb|(ok)?(lab|lch)|color-mix)\(.+\)$/,ko=/^(inset_)?-?((\d+)?\.?(\d+)[a-z]+|0)_-?((\d+)?\.?(\d+)[a-z]+|0)/,fo=/^(url|image|image-set|cross-fade|element|(repeating-)?(linear|radial|conic)-gradient)\(.+\)$/,V=e=>mo.test(e),u=e=>!!e&&!Number.isNaN(Number(e)),$=e=>!!e&&Number.isInteger(Number(e)),se=e=>e.endsWith("%")&&u(e.slice(0,-1)),S=e=>uo.test(e),qe=()=>!0,go=e=>yo.test(e)&&!bo.test(e),le=()=>!1,xo=e=>ko.test(e),vo=e=>fo.test(e),wo=e=>!s(e)&&!n(e),Mo=e=>e.startsWith("@container")&&(e[10]==="/"&&e[11]!==void 0||e[11]==="s"&&e[16]!==void 0&&e.startsWith("-size/",10)||e[11]==="n"&&e[18]!==void 0&&e.startsWith("-normal/",10)),_o=e=>q(e,Ie,le),s=e=>Se.test(e),P=e=>q(e,Pe,go),xe=e=>q(e,Vo,u),zo=e=>q(e,He,qe),Co=e=>q(e,Ge,le),ve=e=>q(e,Le,le),No=e=>q(e,Re,vo),X=e=>q(e,Te,xo),n=e=>Ve.test(e),F=e=>G(e,Pe),Ao=e=>G(e,Ge),we=e=>G(e,Le),$o=e=>G(e,Ie),jo=e=>G(e,Re),J=e=>G(e,Te,!0),So=e=>G(e,He,!0),q=(e,t,o)=>{const r=Se.exec(e);return r?r[1]?t(r[1]):o(r[2]):!1},G=(e,t,o=!1)=>{const r=Ve.exec(e);return r?r[1]?t(r[1]):o:!1},Le=e=>e==="position"||e==="percentage",Re=e=>e==="image"||e==="url",Ie=e=>e==="length"||e==="size"||e==="bg-size",Pe=e=>e==="length",Vo=e=>e==="number",Ge=e=>e==="family-name",He=e=>e==="number"||e==="weight",Te=e=>e==="shadow",qo=()=>{const e=f("color"),t=f("font"),o=f("text"),r=f("font-weight"),i=f("tracking"),l=f("leading"),d=f("breakpoint"),m=f("container"),y=f("spacing"),p=f("radius"),b=f("shadow"),k=f("inset-shadow"),_=f("text-shadow"),N=f("drop-shadow"),L=f("blur"),x=f("perspective"),v=f("aspect"),R=f("ease"),z=f("animate"),W=()=>["auto","avoid","all","avoid-page","page","left","right","column"],H=()=>["center","top","bottom","left","right","top-left","left-top","top-right","right-top","bottom-right","right-bottom","bottom-left","left-bottom"],T=()=>[...H(),n,s],O=()=>["auto","hidden","clip","visible","scroll"],j=()=>["auto","contain","none"],c=()=>[n,s,y],C=()=>[V,"full","auto",...c()],ce=()=>[$,"none","subgrid",n,s],de=()=>["auto",{span:["full",$,n,s]},$,n,s],U=()=>[$,"auto",n,s],pe=()=>["auto","min","max","fr",n,s],ee=()=>["start","end","center","between","around","evenly","stretch","baseline","center-safe","end-safe"],E=()=>["start","end","center","stretch","center-safe","end-safe"],A=()=>["auto",...c()],I=()=>[V,"auto","full","dvw","dvh","lvw","lvh","svw","svh","min","max","fit",...c()],oe=()=>[V,"screen","full","dvw","lvw","svw","min","max","fit",...c()],te=()=>[V,"screen","full","lh","dvh","lvh","svh","min","max","fit",...c()],h=()=>[e,n,s],he=()=>[...H(),we,ve,{position:[n,s]}],me=()=>["no-repeat",{repeat:["","x","y","space","round"]}],ue=()=>["auto","cover","contain",$o,_o,{size:[n,s]}],re=()=>[se,F,P],w=()=>["","none","full",p,n,s],M=()=>["",u,F,P],Z=()=>["solid","dashed","dotted","double"],ye=()=>["normal","multiply","screen","overlay","darken","lighten","color-dodge","color-burn","hard-light","soft-light","difference","exclusion","hue","saturation","color","luminosity"],g=()=>[u,se,we,ve],be=()=>["","none",L,n,s],D=()=>["none",u,n,s],Y=()=>["none",u,n,s],ae=()=>[u,n,s],K=()=>[V,"full",...c()];return{cacheSize:500,theme:{animate:["spin","ping","pulse","bounce"],aspect:["video"],blur:[S],breakpoint:[S],color:[qe],container:[S],"drop-shadow":[S],ease:["in","out","in-out"],font:[wo],"font-weight":["thin","extralight","light","normal","medium","semibold","bold","extrabold","black"],"inset-shadow":[S],leading:["none","tight","snug","normal","relaxed","loose"],perspective:["dramatic","near","normal","midrange","distant","none"],radius:[S],shadow:[S],spacing:["px",u],text:[S],"text-shadow":[S],tracking:["tighter","tight","normal","wide","wider","widest"]},classGroups:{aspect:[{aspect:["auto","square",V,s,n,v]}],container:["container"],"container-type":[{"@container":["","normal","size",n,s]}],"container-named":[Mo],columns:[{columns:[u,s,n,m]}],"break-after":[{"break-after":W()}],"break-before":[{"break-before":W()}],"break-inside":[{"break-inside":["auto","avoid","avoid-page","avoid-column"]}],"box-decoration":[{"box-decoration":["slice","clone"]}],box:[{box:["border","content"]}],display:["block","inline-block","inline","flex","inline-flex","table","inline-table","table-caption","table-cell","table-column","table-column-group","table-footer-group","table-header-group","table-row-group","table-row","flow-root","grid","inline-grid","contents","list-item","hidden"],sr:["sr-only","not-sr-only"],float:[{float:["right","left","none","start","end"]}],clear:[{clear:["left","right","both","none","start","end"]}],isolation:["isolate","isolation-auto"],"object-fit":[{object:["contain","cover","fill","none","scale-down"]}],"object-position":[{object:T()}],overflow:[{overflow:O()}],"overflow-x":[{"overflow-x":O()}],"overflow-y":[{"overflow-y":O()}],overscroll:[{overscroll:j()}],"overscroll-x":[{"overscroll-x":j()}],"overscroll-y":[{"overscroll-y":j()}],position:["static","fixed","absolute","relative","sticky"],inset:[{inset:C()}],"inset-x":[{"inset-x":C()}],"inset-y":[{"inset-y":C()}],start:[{"inset-s":C(),start:C()}],end:[{"inset-e":C(),end:C()}],"inset-bs":[{"inset-bs":C()}],"inset-be":[{"inset-be":C()}],top:[{top:C()}],right:[{right:C()}],bottom:[{bottom:C()}],left:[{left:C()}],visibility:["visible","invisible","collapse"],z:[{z:[$,"auto",n,s]}],basis:[{basis:[V,"full","auto",m,...c()]}],"flex-direction":[{flex:["row","row-reverse","col","col-reverse"]}],"flex-wrap":[{flex:["nowrap","wrap","wrap-reverse"]}],flex:[{flex:[u,V,"auto","initial","none",s]}],grow:[{grow:["",u,n,s]}],shrink:[{shrink:["",u,n,s]}],order:[{order:[$,"first","last","none",n,s]}],"grid-cols":[{"grid-cols":ce()}],"col-start-end":[{col:de()}],"col-start":[{"col-start":U()}],"col-end":[{"col-end":U()}],"grid-rows":[{"grid-rows":ce()}],"row-start-end":[{row:de()}],"row-start":[{"row-start":U()}],"row-end":[{"row-end":U()}],"grid-flow":[{"grid-flow":["row","col","dense","row-dense","col-dense"]}],"auto-cols":[{"auto-cols":pe()}],"auto-rows":[{"auto-rows":pe()}],gap:[{gap:c()}],"gap-x":[{"gap-x":c()}],"gap-y":[{"gap-y":c()}],"justify-content":[{justify:[...ee(),"normal"]}],"justify-items":[{"justify-items":[...E(),"normal"]}],"justify-self":[{"justify-self":["auto",...E()]}],"align-content":[{content:["normal",...ee()]}],"align-items":[{items:[...E(),{baseline:["","last"]}]}],"align-self":[{self:["auto",...E(),{baseline:["","last"]}]}],"place-content":[{"place-content":ee()}],"place-items":[{"place-items":[...E(),"baseline"]}],"place-self":[{"place-self":["auto",...E()]}],p:[{p:c()}],px:[{px:c()}],py:[{py:c()}],ps:[{ps:c()}],pe:[{pe:c()}],pbs:[{pbs:c()}],pbe:[{pbe:c()}],pt:[{pt:c()}],pr:[{pr:c()}],pb:[{pb:c()}],pl:[{pl:c()}],m:[{m:A()}],mx:[{mx:A()}],my:[{my:A()}],ms:[{ms:A()}],me:[{me:A()}],mbs:[{mbs:A()}],mbe:[{mbe:A()}],mt:[{mt:A()}],mr:[{mr:A()}],mb:[{mb:A()}],ml:[{ml:A()}],"space-x":[{"space-x":c()}],"space-x-reverse":["space-x-reverse"],"space-y":[{"space-y":c()}],"space-y-reverse":["space-y-reverse"],size:[{size:I()}],"inline-size":[{inline:["auto",...oe()]}],"min-inline-size":[{"min-inline":["auto",...oe()]}],"max-inline-size":[{"max-inline":["none",...oe()]}],"block-size":[{block:["auto",...te()]}],"min-block-size":[{"min-block":["auto",...te()]}],"max-block-size":[{"max-block":["none",...te()]}],w:[{w:[m,"screen",...I()]}],"min-w":[{"min-w":[m,"screen","none",...I()]}],"max-w":[{"max-w":[m,"screen","none","prose",{screen:[d]},...I()]}],h:[{h:["screen","lh",...I()]}],"min-h":[{"min-h":["screen","lh","none",...I()]}],"max-h":[{"max-h":["screen","lh",...I()]}],"font-size":[{text:["base",o,F,P]}],"font-smoothing":["antialiased","subpixel-antialiased"],"font-style":["italic","not-italic"],"font-weight":[{font:[r,So,zo]}],"font-stretch":[{"font-stretch":["ultra-condensed","extra-condensed","condensed","semi-condensed","normal","semi-expanded","expanded","extra-expanded","ultra-expanded",se,s]}],"font-family":[{font:[Ao,Co,t]}],"font-features":[{"font-features":[s]}],"fvn-normal":["normal-nums"],"fvn-ordinal":["ordinal"],"fvn-slashed-zero":["slashed-zero"],"fvn-figure":["lining-nums","oldstyle-nums"],"fvn-spacing":["proportional-nums","tabular-nums"],"fvn-fraction":["diagonal-fractions","stacked-fractions"],tracking:[{tracking:[i,n,s]}],"line-clamp":[{"line-clamp":[u,"none",n,xe]}],leading:[{leading:[l,...c()]}],"list-image":[{"list-image":["none",n,s]}],"list-style-position":[{list:["inside","outside"]}],"list-style-type":[{list:["disc","decimal","none",n,s]}],"text-alignment":[{text:["left","center","right","justify","start","end"]}],"placeholder-color":[{placeholder:h()}],"text-color":[{text:h()}],"text-decoration":["underline","overline","line-through","no-underline"],"text-decoration-style":[{decoration:[...Z(),"wavy"]}],"text-decoration-thickness":[{decoration:[u,"from-font","auto",n,P]}],"text-decoration-color":[{decoration:h()}],"underline-offset":[{"underline-offset":[u,"auto",n,s]}],"text-transform":["uppercase","lowercase","capitalize","normal-case"],"text-overflow":["truncate","text-ellipsis","text-clip"],"text-wrap":[{text:["wrap","nowrap","balance","pretty"]}],indent:[{indent:c()}],"tab-size":[{tab:[$,n,s]}],"vertical-align":[{align:["baseline","top","middle","bottom","text-top","text-bottom","sub","super",n,s]}],whitespace:[{whitespace:["normal","nowrap","pre","pre-line","pre-wrap","break-spaces"]}],break:[{break:["normal","words","all","keep"]}],wrap:[{wrap:["break-word","anywhere","normal"]}],hyphens:[{hyphens:["none","manual","auto"]}],content:[{content:["none",n,s]}],"bg-attachment":[{bg:["fixed","local","scroll"]}],"bg-clip":[{"bg-clip":["border","padding","content","text"]}],"bg-origin":[{"bg-origin":["border","padding","content"]}],"bg-position":[{bg:he()}],"bg-repeat":[{bg:me()}],"bg-size":[{bg:ue()}],"bg-image":[{bg:["none",{linear:[{to:["t","tr","r","br","b","bl","l","tl"]},$,n,s],radial:["",n,s],conic:[$,n,s]},jo,No]}],"bg-color":[{bg:h()}],"gradient-from-pos":[{from:re()}],"gradient-via-pos":[{via:re()}],"gradient-to-pos":[{to:re()}],"gradient-from":[{from:h()}],"gradient-via":[{via:h()}],"gradient-to":[{to:h()}],rounded:[{rounded:w()}],"rounded-s":[{"rounded-s":w()}],"rounded-e":[{"rounded-e":w()}],"rounded-t":[{"rounded-t":w()}],"rounded-r":[{"rounded-r":w()}],"rounded-b":[{"rounded-b":w()}],"rounded-l":[{"rounded-l":w()}],"rounded-ss":[{"rounded-ss":w()}],"rounded-se":[{"rounded-se":w()}],"rounded-ee":[{"rounded-ee":w()}],"rounded-es":[{"rounded-es":w()}],"rounded-tl":[{"rounded-tl":w()}],"rounded-tr":[{"rounded-tr":w()}],"rounded-br":[{"rounded-br":w()}],"rounded-bl":[{"rounded-bl":w()}],"border-w":[{border:M()}],"border-w-x":[{"border-x":M()}],"border-w-y":[{"border-y":M()}],"border-w-s":[{"border-s":M()}],"border-w-e":[{"border-e":M()}],"border-w-bs":[{"border-bs":M()}],"border-w-be":[{"border-be":M()}],"border-w-t":[{"border-t":M()}],"border-w-r":[{"border-r":M()}],"border-w-b":[{"border-b":M()}],"border-w-l":[{"border-l":M()}],"divide-x":[{"divide-x":M()}],"divide-x-reverse":["divide-x-reverse"],"divide-y":[{"divide-y":M()}],"divide-y-reverse":["divide-y-reverse"],"border-style":[{border:[...Z(),"hidden","none"]}],"divide-style":[{divide:[...Z(),"hidden","none"]}],"border-color":[{border:h()}],"border-color-x":[{"border-x":h()}],"border-color-y":[{"border-y":h()}],"border-color-s":[{"border-s":h()}],"border-color-e":[{"border-e":h()}],"border-color-bs":[{"border-bs":h()}],"border-color-be":[{"border-be":h()}],"border-color-t":[{"border-t":h()}],"border-color-r":[{"border-r":h()}],"border-color-b":[{"border-b":h()}],"border-color-l":[{"border-l":h()}],"divide-color":[{divide:h()}],"outline-style":[{outline:[...Z(),"none","hidden"]}],"outline-offset":[{"outline-offset":[u,n,s]}],"outline-w":[{outline:["",u,F,P]}],"outline-color":[{outline:h()}],shadow:[{shadow:["","none",b,J,X]}],"shadow-color":[{shadow:h()}],"inset-shadow":[{"inset-shadow":["none",k,J,X]}],"inset-shadow-color":[{"inset-shadow":h()}],"ring-w":[{ring:M()}],"ring-w-inset":["ring-inset"],"ring-color":[{ring:h()}],"ring-offset-w":[{"ring-offset":[u,P]}],"ring-offset-color":[{"ring-offset":h()}],"inset-ring-w":[{"inset-ring":M()}],"inset-ring-color":[{"inset-ring":h()}],"text-shadow":[{"text-shadow":["none",_,J,X]}],"text-shadow-color":[{"text-shadow":h()}],opacity:[{opacity:[u,n,s]}],"mix-blend":[{"mix-blend":[...ye(),"plus-darker","plus-lighter"]}],"bg-blend":[{"bg-blend":ye()}],"mask-clip":[{"mask-clip":["border","padding","content","fill","stroke","view"]},"mask-no-clip"],"mask-composite":[{mask:["add","subtract","intersect","exclude"]}],"mask-image-linear-pos":[{"mask-linear":[u]}],"mask-image-linear-from-pos":[{"mask-linear-from":g()}],"mask-image-linear-to-pos":[{"mask-linear-to":g()}],"mask-image-linear-from-color":[{"mask-linear-from":h()}],"mask-image-linear-to-color":[{"mask-linear-to":h()}],"mask-image-t-from-pos":[{"mask-t-from":g()}],"mask-image-t-to-pos":[{"mask-t-to":g()}],"mask-image-t-from-color":[{"mask-t-from":h()}],"mask-image-t-to-color":[{"mask-t-to":h()}],"mask-image-r-from-pos":[{"mask-r-from":g()}],"mask-image-r-to-pos":[{"mask-r-to":g()}],"mask-image-r-from-color":[{"mask-r-from":h()}],"mask-image-r-to-color":[{"mask-r-to":h()}],"mask-image-b-from-pos":[{"mask-b-from":g()}],"mask-image-b-to-pos":[{"mask-b-to":g()}],"mask-image-b-from-color":[{"mask-b-from":h()}],"mask-image-b-to-color":[{"mask-b-to":h()}],"mask-image-l-from-pos":[{"mask-l-from":g()}],"mask-image-l-to-pos":[{"mask-l-to":g()}],"mask-image-l-from-color":[{"mask-l-from":h()}],"mask-image-l-to-color":[{"mask-l-to":h()}],"mask-image-x-from-pos":[{"mask-x-from":g()}],"mask-image-x-to-pos":[{"mask-x-to":g()}],"mask-image-x-from-color":[{"mask-x-from":h()}],"mask-image-x-to-color":[{"mask-x-to":h()}],"mask-image-y-from-pos":[{"mask-y-from":g()}],"mask-image-y-to-pos":[{"mask-y-to":g()}],"mask-image-y-from-color":[{"mask-y-from":h()}],"mask-image-y-to-color":[{"mask-y-to":h()}],"mask-image-radial":[{"mask-radial":[n,s]}],"mask-image-radial-from-pos":[{"mask-radial-from":g()}],"mask-image-radial-to-pos":[{"mask-radial-to":g()}],"mask-image-radial-from-color":[{"mask-radial-from":h()}],"mask-image-radial-to-color":[{"mask-radial-to":h()}],"mask-image-radial-shape":[{"mask-radial":["circle","ellipse"]}],"mask-image-radial-size":[{"mask-radial":[{closest:["side","corner"],farthest:["side","corner"]}]}],"mask-image-radial-pos":[{"mask-radial-at":H()}],"mask-image-conic-pos":[{"mask-conic":[u]}],"mask-image-conic-from-pos":[{"mask-conic-from":g()}],"mask-image-conic-to-pos":[{"mask-conic-to":g()}],"mask-image-conic-from-color":[{"mask-conic-from":h()}],"mask-image-conic-to-color":[{"mask-conic-to":h()}],"mask-mode":[{mask:["alpha","luminance","match"]}],"mask-origin":[{"mask-origin":["border","padding","content","fill","stroke","view"]}],"mask-position":[{mask:he()}],"mask-repeat":[{mask:me()}],"mask-size":[{mask:ue()}],"mask-type":[{"mask-type":["alpha","luminance"]}],"mask-image":[{mask:["none",n,s]}],filter:[{filter:["","none",n,s]}],blur:[{blur:be()}],brightness:[{brightness:[u,n,s]}],contrast:[{contrast:[u,n,s]}],"drop-shadow":[{"drop-shadow":["","none",N,J,X]}],"drop-shadow-color":[{"drop-shadow":h()}],grayscale:[{grayscale:["",u,n,s]}],"hue-rotate":[{"hue-rotate":[u,n,s]}],invert:[{invert:["",u,n,s]}],saturate:[{saturate:[u,n,s]}],sepia:[{sepia:["",u,n,s]}],"backdrop-filter":[{"backdrop-filter":["","none",n,s]}],"backdrop-blur":[{"backdrop-blur":be()}],"backdrop-brightness":[{"backdrop-brightness":[u,n,s]}],"backdrop-contrast":[{"backdrop-contrast":[u,n,s]}],"backdrop-grayscale":[{"backdrop-grayscale":["",u,n,s]}],"backdrop-hue-rotate":[{"backdrop-hue-rotate":[u,n,s]}],"backdrop-invert":[{"backdrop-invert":["",u,n,s]}],"backdrop-opacity":[{"backdrop-opacity":[u,n,s]}],"backdrop-saturate":[{"backdrop-saturate":[u,n,s]}],"backdrop-sepia":[{"backdrop-sepia":["",u,n,s]}],"border-collapse":[{border:["collapse","separate"]}],"border-spacing":[{"border-spacing":c()}],"border-spacing-x":[{"border-spacing-x":c()}],"border-spacing-y":[{"border-spacing-y":c()}],"table-layout":[{table:["auto","fixed"]}],caption:[{caption:["top","bottom"]}],transition:[{transition:["","all","colors","opacity","shadow","transform","none",n,s]}],"transition-behavior":[{transition:["normal","discrete"]}],duration:[{duration:[u,"initial",n,s]}],ease:[{ease:["linear","initial",R,n,s]}],delay:[{delay:[u,n,s]}],animate:[{animate:["none",z,n,s]}],backface:[{backface:["hidden","visible"]}],perspective:[{perspective:[x,n,s]}],"perspective-origin":[{"perspective-origin":T()}],rotate:[{rotate:D()}],"rotate-x":[{"rotate-x":D()}],"rotate-y":[{"rotate-y":D()}],"rotate-z":[{"rotate-z":D()}],scale:[{scale:Y()}],"scale-x":[{"scale-x":Y()}],"scale-y":[{"scale-y":Y()}],"scale-z":[{"scale-z":Y()}],"scale-3d":["scale-3d"],skew:[{skew:ae()}],"skew-x":[{"skew-x":ae()}],"skew-y":[{"skew-y":ae()}],transform:[{transform:[n,s,"","none","gpu","cpu"]}],"transform-origin":[{origin:T()}],"transform-style":[{transform:["3d","flat"]}],translate:[{translate:K()}],"translate-x":[{"translate-x":K()}],"translate-y":[{"translate-y":K()}],"translate-z":[{"translate-z":K()}],"translate-none":["translate-none"],zoom:[{zoom:[$,n,s]}],accent:[{accent:h()}],appearance:[{appearance:["none","auto"]}],"caret-color":[{caret:h()}],"color-scheme":[{scheme:["normal","dark","light","light-dark","only-dark","only-light"]}],cursor:[{cursor:["auto","default","pointer","wait","text","move","help","not-allowed","none","context-menu","progress","cell","crosshair","vertical-text","alias","copy","no-drop","grab","grabbing","all-scroll","col-resize","row-resize","n-resize","e-resize","s-resize","w-resize","ne-resize","nw-resize","se-resize","sw-resize","ew-resize","ns-resize","nesw-resize","nwse-resize","zoom-in","zoom-out",n,s]}],"field-sizing":[{"field-sizing":["fixed","content"]}],"pointer-events":[{"pointer-events":["auto","none"]}],resize:[{resize:["none","","y","x"]}],"scroll-behavior":[{scroll:["auto","smooth"]}],"scrollbar-thumb-color":[{"scrollbar-thumb":h()}],"scrollbar-track-color":[{"scrollbar-track":h()}],"scrollbar-gutter":[{"scrollbar-gutter":["auto","stable","both"]}],"scrollbar-w":[{scrollbar:["auto","thin","none"]}],"scroll-m":[{"scroll-m":c()}],"scroll-mx":[{"scroll-mx":c()}],"scroll-my":[{"scroll-my":c()}],"scroll-ms":[{"scroll-ms":c()}],"scroll-me":[{"scroll-me":c()}],"scroll-mbs":[{"scroll-mbs":c()}],"scroll-mbe":[{"scroll-mbe":c()}],"scroll-mt":[{"scroll-mt":c()}],"scroll-mr":[{"scroll-mr":c()}],"scroll-mb":[{"scroll-mb":c()}],"scroll-ml":[{"scroll-ml":c()}],"scroll-p":[{"scroll-p":c()}],"scroll-px":[{"scroll-px":c()}],"scroll-py":[{"scroll-py":c()}],"scroll-ps":[{"scroll-ps":c()}],"scroll-pe":[{"scroll-pe":c()}],"scroll-pbs":[{"scroll-pbs":c()}],"scroll-pbe":[{"scroll-pbe":c()}],"scroll-pt":[{"scroll-pt":c()}],"scroll-pr":[{"scroll-pr":c()}],"scroll-pb":[{"scroll-pb":c()}],"scroll-pl":[{"scroll-pl":c()}],"snap-align":[{snap:["start","end","center","align-none"]}],"snap-stop":[{snap:["normal","always"]}],"snap-type":[{snap:["none","x","y","both"]}],"snap-strictness":[{snap:["mandatory","proximity"]}],touch:[{touch:["auto","none","manipulation"]}],"touch-x":[{"touch-pan":["x","left","right"]}],"touch-y":[{"touch-pan":["y","up","down"]}],"touch-pz":["touch-pinch-zoom"],select:[{select:["none","text","all","auto"]}],"will-change":[{"will-change":["auto","scroll","contents","transform",n,s]}],fill:[{fill:["none",...h()]}],"stroke-w":[{stroke:[u,F,P,xe]}],stroke:[{stroke:["none",...h()]}],"forced-color-adjust":[{"forced-color-adjust":["auto","none"]}]},conflictingClassGroups:{"container-named":["container-type"],overflow:["overflow-x","overflow-y"],overscroll:["overscroll-x","overscroll-y"],inset:["inset-x","inset-y","inset-bs","inset-be","start","end","top","right","bottom","left"],"inset-x":["right","left"],"inset-y":["top","bottom"],flex:["basis","grow","shrink"],gap:["gap-x","gap-y"],p:["px","py","ps","pe","pbs","pbe","pt","pr","pb","pl"],px:["pr","pl"],py:["pt","pb"],m:["mx","my","ms","me","mbs","mbe","mt","mr","mb","ml"],mx:["mr","ml"],my:["mt","mb"],size:["w","h"],"font-size":["leading"],"fvn-normal":["fvn-ordinal","fvn-slashed-zero","fvn-figure","fvn-spacing","fvn-fraction"],"fvn-ordinal":["fvn-normal"],"fvn-slashed-zero":["fvn-normal"],"fvn-figure":["fvn-normal"],"fvn-spacing":["fvn-normal"],"fvn-fraction":["fvn-normal"],"line-clamp":["display","overflow"],rounded:["rounded-s","rounded-e","rounded-t","rounded-r","rounded-b","rounded-l","rounded-ss","rounded-se","rounded-ee","rounded-es","rounded-tl","rounded-tr","rounded-br","rounded-bl"],"rounded-s":["rounded-ss","rounded-es"],"rounded-e":["rounded-se","rounded-ee"],"rounded-t":["rounded-tl","rounded-tr"],"rounded-r":["rounded-tr","rounded-br"],"rounded-b":["rounded-br","rounded-bl"],"rounded-l":["rounded-tl","rounded-bl"],"border-spacing":["border-spacing-x","border-spacing-y"],"border-w":["border-w-x","border-w-y","border-w-s","border-w-e","border-w-bs","border-w-be","border-w-t","border-w-r","border-w-b","border-w-l"],"border-w-x":["border-w-r","border-w-l"],"border-w-y":["border-w-t","border-w-b"],"border-color":["border-color-x","border-color-y","border-color-s","border-color-e","border-color-bs","border-color-be","border-color-t","border-color-r","border-color-b","border-color-l"],"border-color-x":["border-color-r","border-color-l"],"border-color-y":["border-color-t","border-color-b"],translate:["translate-x","translate-y","translate-none"],"translate-none":["translate","translate-x","translate-y","translate-z"],"scroll-m":["scroll-mx","scroll-my","scroll-ms","scroll-me","scroll-mbs","scroll-mbe","scroll-mt","scroll-mr","scroll-mb","scroll-ml"],"scroll-mx":["scroll-mr","scroll-ml"],"scroll-my":["scroll-mt","scroll-mb"],"scroll-p":["scroll-px","scroll-py","scroll-ps","scroll-pe","scroll-pbs","scroll-pbe","scroll-pt","scroll-pr","scroll-pb","scroll-pl"],"scroll-px":["scroll-pr","scroll-pl"],"scroll-py":["scroll-pt","scroll-pb"],touch:["touch-x","touch-y","touch-pz"],"touch-x":["touch"],"touch-y":["touch"],"touch-pz":["touch"]},conflictingClassGroupModifiers:{"font-size":["leading"]},postfixLookupClassGroups:["container-type"],orderSensitiveModifiers:["*","**","after","backdrop","before","details-content","file","first-letter","first-line","marker","placeholder","selection"]}},tr=po(qo),Me=e=>typeof e=="boolean"?`${e}`:e===0?"0":e,_e=Ee,rr=(e,t)=>o=>{var r;if((t==null?void 0:t.variants)==null)return _e(e,o==null?void 0:o.class,o==null?void 0:o.className);const{variants:i,defaultVariants:l}=t,d=Object.keys(i).map(p=>{const b=o==null?void 0:o[p],k=l==null?void 0:l[p];if(b===null)return null;const _=Me(b)||Me(k);return i[p][_]}),m=o&&Object.entries(o).reduce((p,b)=>{let[k,_]=b;return _===void 0||(p[k]=_),p},{}),y=t==null||(r=t.compoundVariants)===null||r===void 0?void 0:r.reduce((p,b)=>{let{class:k,className:_,...N}=b;return Object.entries(N).every(L=>{let[x,v]=L;return Array.isArray(v)?v.includes({...l,...m}[x]):{...l,...m}[x]===v})?[...p,k,_]:p},[]);return _e(e,d,y,o==null?void 0:o.class,o==null?void 0:o.className)};/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Lo=e=>e.replace(/([a-z0-9])([A-Z])/g,"$1-$2").toLowerCase(),Ro=e=>e.replace(/^([A-Z])|[\s-_]+(\w)/g,(t,o,r)=>r?r.toUpperCase():o.toLowerCase()),ze=e=>{const t=Ro(e);return t.charAt(0).toUpperCase()+t.slice(1)},Oe=(...e)=>e.filter((t,o,r)=>!!t&&t.trim()!==""&&r.indexOf(t)===o).join(" ").trim(),Io=e=>{for(const t in e)if(t.startsWith("aria-")||t==="role"||t==="title")return!0};/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */var Po={xmlns:"http://www.w3.org/2000/svg",width:24,height:24,viewBox:"0 0 24 24",fill:"none",stroke:"currentColor",strokeWidth:2,strokeLinecap:"round",strokeLinejoin:"round"};/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Go=B.forwardRef(({color:e="currentColor",size:t=24,strokeWidth:o=2,absoluteStrokeWidth:r,className:i="",children:l,iconNode:d,...m},y)=>B.createElement("svg",{ref:y,...Po,width:t,height:t,stroke:e,strokeWidth:r?Number(o)*24/Number(t):o,className:Oe("lucide",i),...!l&&!Io(m)&&{"aria-hidden":"true"},...m},[...d.map(([p,b])=>B.createElement(p,b)),...Array.isArray(l)?l:[l]]));/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const a=(e,t)=>{const o=B.forwardRef(({className:r,...i},l)=>B.createElement(Go,{ref:l,iconNode:t,className:Oe(`lucide-${Lo(ze(e))}`,`lucide-${e}`,r),...i}));return o.displayName=ze(e),o};/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Ho=[["path",{d:"m12 19-7-7 7-7",key:"1l729n"}],["path",{d:"M19 12H5",key:"x3x0zl"}]],ar=a("arrow-left",Ho);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const To=[["path",{d:"M5 12h14",key:"1ays0h"}],["path",{d:"m12 5 7 7-7 7",key:"xquz4c"}]],sr=a("arrow-right",To);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Oo=[["path",{d:"m5 12 7-7 7 7",key:"hav0vg"}],["path",{d:"M12 19V5",key:"x0mq9r"}]],nr=a("arrow-up",Oo);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Eo=[["path",{d:"M10.268 21a2 2 0 0 0 3.464 0",key:"vwvbt9"}],["path",{d:"M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326",key:"11g9vi"}]],ir=a("bell",Eo);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Wo=[["path",{d:"M12 7v14",key:"1akyts"}],["path",{d:"M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z",key:"ruj8y"}]],lr=a("book-open",Wo);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Fo=[["path",{d:"M12 8V4H8",key:"hb8ula"}],["rect",{width:"16",height:"12",x:"4",y:"8",rx:"2",key:"enze0r"}],["path",{d:"M2 14h2",key:"vft8re"}],["path",{d:"M20 14h2",key:"4cs60a"}],["path",{d:"M15 13v2",key:"1xurst"}],["path",{d:"M9 13v2",key:"rq6x2g"}]],cr=a("bot",Fo);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Bo=[["path",{d:"M12 5a3 3 0 1 0-5.997.125 4 4 0 0 0-2.526 5.77 4 4 0 0 0 .556 6.588A4 4 0 1 0 12 18Z",key:"l5xja"}],["path",{d:"M9 13a4.5 4.5 0 0 0 3-4",key:"10igwf"}],["path",{d:"M6.003 5.125A3 3 0 0 0 6.401 6.5",key:"105sqy"}],["path",{d:"M3.477 10.896a4 4 0 0 1 .585-.396",key:"ql3yin"}],["path",{d:"M6 18a4 4 0 0 1-1.967-.516",key:"2e4loj"}],["path",{d:"M12 13h4",key:"1ku699"}],["path",{d:"M12 18h6a2 2 0 0 1 2 2v1",key:"105ag5"}],["path",{d:"M12 8h8",key:"1lhi5i"}],["path",{d:"M16 8V5a2 2 0 0 1 2-2",key:"u6izg6"}],["circle",{cx:"16",cy:"13",r:".5",key:"ry7gng"}],["circle",{cx:"18",cy:"3",r:".5",key:"1aiba7"}],["circle",{cx:"20",cy:"21",r:".5",key:"yhc1fs"}],["circle",{cx:"20",cy:"8",r:".5",key:"1e43v0"}]],dr=a("brain-circuit",Bo);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Uo=[["path",{d:"M12 18V5",key:"adv99a"}],["path",{d:"M15 13a4.17 4.17 0 0 1-3-4 4.17 4.17 0 0 1-3 4",key:"1e3is1"}],["path",{d:"M17.598 6.5A3 3 0 1 0 12 5a3 3 0 1 0-5.598 1.5",key:"1gqd8o"}],["path",{d:"M17.997 5.125a4 4 0 0 1 2.526 5.77",key:"iwvgf7"}],["path",{d:"M18 18a4 4 0 0 0 2-7.464",key:"efp6ie"}],["path",{d:"M19.967 17.483A4 4 0 1 1 12 18a4 4 0 1 1-7.967-.517",key:"1gq6am"}],["path",{d:"M6 18a4 4 0 0 1-2-7.464",key:"k1g0md"}],["path",{d:"M6.003 5.125a4 4 0 0 0-2.526 5.77",key:"q97ue3"}]],pr=a("brain",Uo);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Zo=[["path",{d:"M12 20v-9",key:"1qisl0"}],["path",{d:"M14 7a4 4 0 0 1 4 4v3a6 6 0 0 1-12 0v-3a4 4 0 0 1 4-4z",key:"uouzyp"}],["path",{d:"M14.12 3.88 16 2",key:"qol33r"}],["path",{d:"M21 21a4 4 0 0 0-3.81-4",key:"1b0z45"}],["path",{d:"M21 5a4 4 0 0 1-3.55 3.97",key:"5cxbf6"}],["path",{d:"M22 13h-4",key:"1jl80f"}],["path",{d:"M3 21a4 4 0 0 1 3.81-4",key:"1fjd4g"}],["path",{d:"M3 5a4 4 0 0 0 3.55 3.97",key:"1d7oge"}],["path",{d:"M6 13H2",key:"82j7cp"}],["path",{d:"m8 2 1.88 1.88",key:"fmnt4t"}],["path",{d:"M9 7.13V6a3 3 0 1 1 6 0v1.13",key:"1vgav8"}]],hr=a("bug",Zo);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Do=[["path",{d:"M20 6 9 17l-5-5",key:"1gmf2c"}]],mr=a("check",Do);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Yo=[["path",{d:"m6 9 6 6 6-6",key:"qrunsl"}]],ur=a("chevron-down",Yo);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Ko=[["path",{d:"m9 18 6-6-6-6",key:"mthhwq"}]],yr=a("chevron-right",Ko);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Xo=[["path",{d:"m18 15-6-6-6 6",key:"153udz"}]],br=a("chevron-up",Xo);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Jo=[["circle",{cx:"12",cy:"12",r:"10",key:"1mglay"}],["line",{x1:"12",x2:"12",y1:"8",y2:"12",key:"1pkeuh"}],["line",{x1:"12",x2:"12.01",y1:"16",y2:"16",key:"4dfq90"}]],kr=a("circle-alert",Jo);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Qo=[["circle",{cx:"12",cy:"12",r:"10",key:"1mglay"}],["path",{d:"m9 12 2 2 4-4",key:"dzmm74"}]],fr=a("circle-check",Qo);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const et=[["circle",{cx:"12",cy:"12",r:"10",key:"1mglay"}]],gr=a("circle",et);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const ot=[["path",{d:"M12 6v6l4 2",key:"mmk7yg"}],["circle",{cx:"12",cy:"12",r:"10",key:"1mglay"}]],xr=a("clock",ot);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const tt=[["path",{d:"m18 16 4-4-4-4",key:"1inbqp"}],["path",{d:"m6 8-4 4 4 4",key:"15zrgr"}],["path",{d:"m14.5 4-5 16",key:"e7oirm"}]],vr=a("code-xml",tt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const rt=[["path",{d:"M15 6v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3V6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3",key:"11bfej"}]],wr=a("command",rt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const at=[["rect",{width:"14",height:"14",x:"8",y:"8",rx:"2",ry:"2",key:"17jyea"}],["path",{d:"M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2",key:"zix9uf"}]],Mr=a("copy",at);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const st=[["path",{d:"M12 20v2",key:"1lh1kg"}],["path",{d:"M12 2v2",key:"tus03m"}],["path",{d:"M17 20v2",key:"1rnc9c"}],["path",{d:"M17 2v2",key:"11trls"}],["path",{d:"M2 12h2",key:"1t8f8n"}],["path",{d:"M2 17h2",key:"7oei6x"}],["path",{d:"M2 7h2",key:"asdhe0"}],["path",{d:"M20 12h2",key:"1q8mjw"}],["path",{d:"M20 17h2",key:"1fpfkl"}],["path",{d:"M20 7h2",key:"1o8tra"}],["path",{d:"M7 20v2",key:"4gnj0m"}],["path",{d:"M7 2v2",key:"1i4yhu"}],["rect",{x:"4",y:"4",width:"16",height:"16",rx:"2",key:"1vbyd7"}],["rect",{x:"8",y:"8",width:"8",height:"8",rx:"1",key:"z9xiuo"}]];a("cpu",st);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const nt=[["ellipse",{cx:"12",cy:"5",rx:"9",ry:"3",key:"msslwz"}],["path",{d:"M3 5V19A9 3 0 0 0 21 19V5",key:"1wlel7"}],["path",{d:"M3 12A9 3 0 0 0 21 12",key:"mv7ke4"}]],_r=a("database",nt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const it=[["path",{d:"M12 15V3",key:"m9g1x1"}],["path",{d:"M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4",key:"ih7n3h"}],["path",{d:"m7 10 5 5 5-5",key:"brsn70"}]],zr=a("download",it);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const lt=[["path",{d:"M15 3h6v6",key:"1q9fwt"}],["path",{d:"M10 14 21 3",key:"gplh6r"}],["path",{d:"M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6",key:"a6xqqp"}]],Cr=a("external-link",lt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const ct=[["path",{d:"M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49",key:"ct8e1f"}],["path",{d:"M14.084 14.158a3 3 0 0 1-4.242-4.242",key:"151rxh"}],["path",{d:"M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143",key:"13bj9a"}],["path",{d:"m2 2 20 20",key:"1ooewy"}]],Nr=a("eye-off",ct);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const dt=[["path",{d:"M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0",key:"1nclc0"}],["circle",{cx:"12",cy:"12",r:"3",key:"1v7zrd"}]],Ar=a("eye",dt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const pt=[["path",{d:"M4 22h14a2 2 0 0 0 2-2V7l-5-5H6a2 2 0 0 0-2 2v4",key:"1pf5j1"}],["path",{d:"M14 2v4a2 2 0 0 0 2 2h4",key:"tnqrlb"}],["path",{d:"m5 12-3 3 3 3",key:"oke12k"}],["path",{d:"m9 18 3-3-3-3",key:"112psh"}]],$r=a("file-code-2",pt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const ht=[["path",{d:"M10 12.5 8 15l2 2.5",key:"1tg20x"}],["path",{d:"m14 12.5 2 2.5-2 2.5",key:"yinavb"}],["path",{d:"M14 2v4a2 2 0 0 0 2 2h4",key:"tnqrlb"}],["path",{d:"M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7z",key:"1mlx9k"}]],jr=a("file-code",ht);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const mt=[["path",{d:"M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z",key:"1rqfz7"}],["path",{d:"M14 2v4a2 2 0 0 0 2 2h4",key:"tnqrlb"}],["circle",{cx:"10",cy:"12",r:"2",key:"737tya"}],["path",{d:"m20 17-1.296-1.296a2.41 2.41 0 0 0-3.408 0L9 22",key:"wt3hpn"}]],Sr=a("file-image",mt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const ut=[["path",{d:"M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z",key:"1rqfz7"}],["path",{d:"M14 2v4a2 2 0 0 0 2 2h4",key:"tnqrlb"}],["path",{d:"M10 9H8",key:"b1mrlr"}],["path",{d:"M16 13H8",key:"t4e002"}],["path",{d:"M16 17H8",key:"z1uh3a"}]],Vr=a("file-text",ut);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const yt=[["path",{d:"M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z",key:"1kt360"}],["path",{d:"M2 10h20",key:"1ir3d8"}]],qr=a("folder-closed",yt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const bt=[["path",{d:"m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2",key:"usdka0"}]],Lr=a("folder-open",bt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const kt=[["path",{d:"M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z",key:"1kt360"}]],Rr=a("folder",kt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const ft=[["path",{d:"M10 20a1 1 0 0 0 .553.895l2 1A1 1 0 0 0 14 21v-7a2 2 0 0 1 .517-1.341L21.74 4.67A1 1 0 0 0 21 3H3a1 1 0 0 0-.742 1.67l7.225 7.989A2 2 0 0 1 10 14z",key:"sc7q7i"}]],Ir=a("funnel",ft);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const gt=[["line",{x1:"6",x2:"6",y1:"3",y2:"15",key:"17qcm7"}],["circle",{cx:"18",cy:"6",r:"3",key:"1h7g24"}],["circle",{cx:"6",cy:"18",r:"3",key:"fqmcym"}],["path",{d:"M18 9a9 9 0 0 1-9 9",key:"n2h4wq"}]],Pr=a("git-branch",gt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const xt=[["circle",{cx:"12",cy:"12",r:"10",key:"1mglay"}],["path",{d:"M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20",key:"13o1zl"}],["path",{d:"M2 12h20",key:"9i4pu4"}]],Gr=a("globe",xt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const vt=[["path",{d:"m15.5 7.5 2.3 2.3a1 1 0 0 0 1.4 0l2.1-2.1a1 1 0 0 0 0-1.4L19 4",key:"g0fldk"}],["path",{d:"m21 2-9.6 9.6",key:"1j0ho8"}],["circle",{cx:"7.5",cy:"15.5",r:"5.5",key:"yqb3hr"}]],Hr=a("key",vt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const wt=[["path",{d:"M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z",key:"zw3jo"}],["path",{d:"M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12",key:"1wduqc"}],["path",{d:"M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17",key:"kqbvx6"}]],Tr=a("layers",wt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Mt=[["path",{d:"M9 17H7A5 5 0 0 1 7 7h2",key:"8i5ue5"}],["path",{d:"M15 7h2a5 5 0 1 1 0 10h-2",key:"1b9ql8"}],["line",{x1:"8",x2:"16",y1:"12",y2:"12",key:"1jonct"}]],Or=a("link-2",Mt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const _t=[["path",{d:"M21 12a9 9 0 1 1-6.219-8.56",key:"13zald"}]],Er=a("loader-circle",_t);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const zt=[["path",{d:"m16 17 5-5-5-5",key:"1bji2h"}],["path",{d:"M21 12H9",key:"dn1m92"}],["path",{d:"M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4",key:"1uf3rs"}]],Wr=a("log-out",zt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Ct=[["rect",{width:"18",height:"11",x:"3",y:"11",rx:"2",ry:"2",key:"1w4ew1"}],["path",{d:"M7 11V7a5 5 0 0 1 10 0v4",key:"fwvmzm"}]],Fr=a("lock",Ct);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Nt=[["path",{d:"M14.106 5.553a2 2 0 0 0 1.788 0l3.659-1.83A1 1 0 0 1 21 4.619v12.764a1 1 0 0 1-.553.894l-4.553 2.277a2 2 0 0 1-1.788 0l-4.212-2.106a2 2 0 0 0-1.788 0l-3.659 1.83A1 1 0 0 1 3 19.381V6.618a1 1 0 0 1 .553-.894l4.553-2.277a2 2 0 0 1 1.788 0z",key:"169xi5"}],["path",{d:"M15 5.764v15",key:"1pn4in"}],["path",{d:"M9 3.236v15",key:"1uimfh"}]],Br=a("map",Nt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const At=[["path",{d:"M22 17a2 2 0 0 1-2 2H6.828a2 2 0 0 0-1.414.586l-2.202 2.202A.71.71 0 0 1 2 21.286V5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2z",key:"18887p"}]],Ur=a("message-square",At);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const $t=[["rect",{x:"16",y:"16",width:"6",height:"6",rx:"1",key:"4q2zg0"}],["rect",{x:"2",y:"16",width:"6",height:"6",rx:"1",key:"8cvhb9"}],["rect",{x:"9",y:"2",width:"6",height:"6",rx:"1",key:"1egb70"}],["path",{d:"M5 16v-3a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3",key:"1jsf9p"}],["path",{d:"M12 12V8",key:"2874zd"}]],Zr=a("network",$t);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const jt=[["path",{d:"M12 22a1 1 0 0 1 0-20 10 9 0 0 1 10 9 5 5 0 0 1-5 5h-2.25a1.75 1.75 0 0 0-1.4 2.8l.3.4a1.75 1.75 0 0 1-1.4 2.8z",key:"e79jfc"}],["circle",{cx:"13.5",cy:"6.5",r:".5",fill:"currentColor",key:"1okk4w"}],["circle",{cx:"17.5",cy:"10.5",r:".5",fill:"currentColor",key:"f64h9f"}],["circle",{cx:"6.5",cy:"12.5",r:".5",fill:"currentColor",key:"qy21gx"}],["circle",{cx:"8.5",cy:"7.5",r:".5",fill:"currentColor",key:"fotxhn"}]],Dr=a("palette",jt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const St=[["rect",{x:"14",y:"3",width:"5",height:"18",rx:"1",key:"kaeet6"}],["rect",{x:"5",y:"3",width:"5",height:"18",rx:"1",key:"1wsw3u"}]],Yr=a("pause",St);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Vt=[["path",{d:"M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z",key:"10ikf1"}]],Kr=a("play",Vt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const qt=[["path",{d:"M5 12h14",key:"1ays0h"}],["path",{d:"M12 5v14",key:"s699le"}]],Xr=a("plus",qt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Lt=[["path",{d:"M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8",key:"v9h5vc"}],["path",{d:"M21 3v5h-5",key:"1q7to0"}],["path",{d:"M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16",key:"3uifl3"}],["path",{d:"M8 16H3v5",key:"1cv678"}]],Jr=a("refresh-cw",Lt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Rt=[["path",{d:"M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z",key:"m3kijz"}],["path",{d:"m12 15-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z",key:"1fmvmk"}],["path",{d:"M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0",key:"1f8sc4"}],["path",{d:"M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5",key:"qeys4"}]],Qr=a("rocket",Rt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const It=[["path",{d:"M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8",key:"1357e3"}],["path",{d:"M3 3v5h5",key:"1xhq8a"}]],ea=a("rotate-ccw",It);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Pt=[["path",{d:"M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",key:"1c8476"}],["path",{d:"M17 21v-7a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v7",key:"1ydtos"}],["path",{d:"M7 3v4a1 1 0 0 0 1 1h7",key:"t51u73"}]],oa=a("save",Pt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Gt=[["path",{d:"m21 21-4.34-4.34",key:"14j7rj"}],["circle",{cx:"11",cy:"11",r:"8",key:"4ej97u"}]],ta=a("search",Gt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Ht=[["path",{d:"M14 17H5",key:"gfn3mx"}],["path",{d:"M19 7h-9",key:"6i9tg"}],["circle",{cx:"17",cy:"17",r:"3",key:"18b49y"}],["circle",{cx:"7",cy:"7",r:"3",key:"dfmy0x"}]],ra=a("settings-2",Ht);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Tt=[["path",{d:"M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915",key:"1i5ecw"}],["circle",{cx:"12",cy:"12",r:"3",key:"1v7zrd"}]],aa=a("settings",Tt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Ot=[["path",{d:"M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z",key:"oel41y"}]],sa=a("shield",Ot);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Et=[["path",{d:"M10 5H3",key:"1qgfaw"}],["path",{d:"M12 19H3",key:"yhmn1j"}],["path",{d:"M14 3v4",key:"1sua03"}],["path",{d:"M16 17v4",key:"1q0r14"}],["path",{d:"M21 12h-9",key:"1o4lsq"}],["path",{d:"M21 19h-5",key:"1rlt1p"}],["path",{d:"M21 5h-7",key:"1oszz2"}],["path",{d:"M8 10v4",key:"tgpxqk"}],["path",{d:"M8 12H3",key:"a7s4jb"}]],na=a("sliders-horizontal",Et);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Wt=[["path",{d:"M11.017 2.814a1 1 0 0 1 1.966 0l1.051 5.558a2 2 0 0 0 1.594 1.594l5.558 1.051a1 1 0 0 1 0 1.966l-5.558 1.051a2 2 0 0 0-1.594 1.594l-1.051 5.558a1 1 0 0 1-1.966 0l-1.051-5.558a2 2 0 0 0-1.594-1.594l-5.558-1.051a1 1 0 0 1 0-1.966l5.558-1.051a2 2 0 0 0 1.594-1.594z",key:"1s2grr"}],["path",{d:"M20 2v4",key:"1rf3ol"}],["path",{d:"M22 4h-4",key:"gwowj6"}],["circle",{cx:"4",cy:"20",r:"2",key:"6kqj1y"}]],ia=a("sparkles",Wt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Ft=[["path",{d:"M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z",key:"r04s7s"}]],la=a("star",Ft);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Bt=[["path",{d:"M12 19h8",key:"baeox8"}],["path",{d:"m4 17 6-6-6-6",key:"1yngyt"}]],ca=a("terminal",Bt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Ut=[["path",{d:"M14 4v10.54a4 4 0 1 1-4 0V4a2 2 0 0 1 4 0Z",key:"17jzev"}]],da=a("thermometer",Ut);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Zt=[["path",{d:"M10 11v6",key:"nco0om"}],["path",{d:"M14 11v6",key:"outv1u"}],["path",{d:"M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6",key:"miytrc"}],["path",{d:"M3 6h18",key:"d0wm0j"}],["path",{d:"M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2",key:"e791ji"}]],pa=a("trash-2",Zt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Dt=[["path",{d:"M16 7h6v6",key:"box55l"}],["path",{d:"m22 7-8.5 8.5-5-5L2 17",key:"1t1m79"}]],ha=a("trending-up",Dt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Yt=[["path",{d:"m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3",key:"wmoenq"}],["path",{d:"M12 9v4",key:"juzpu7"}],["path",{d:"M12 17h.01",key:"p32p05"}]],ma=a("triangle-alert",Yt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Kt=[["path",{d:"M12 3v12",key:"1x0j5s"}],["path",{d:"m17 8-5-5-5 5",key:"7q97r8"}],["path",{d:"M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4",key:"ih7n3h"}]],ua=a("upload",Kt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Xt=[["path",{d:"M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2",key:"975kel"}],["circle",{cx:"12",cy:"7",r:"4",key:"17ys0d"}]],ya=a("user",Xt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Jt=[["path",{d:"M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.106-3.105c.32-.322.863-.22.983.218a6 6 0 0 1-8.259 7.057l-7.91 7.91a1 1 0 0 1-2.999-3l7.91-7.91a6 6 0 0 1 7.057-8.259c.438.12.54.662.219.984z",key:"1ngwbx"}]],ba=a("wrench",Jt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const Qt=[["path",{d:"M18 6 6 18",key:"1bl5f8"}],["path",{d:"m6 6 12 12",key:"d8bk6v"}]],ka=a("x",Qt);/**
 * @license lucide-react v0.545.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const er=[["path",{d:"M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z",key:"1xq2db"}]],fa=a("zap",er);export{Zr as $,nr as A,pr as B,ur as C,jr as D,Cr as E,qr as F,Gr as G,Sr as H,Vr as I,zr as J,Rr as K,Er as L,Br as M,_r as N,ha as O,Xr as P,la as Q,ea as R,aa as S,ca as T,ua as U,lr as V,ba as W,ka as X,Ir as Y,fa as Z,ma as _,rr as a,Yr as a0,gr as a1,Hr as a2,xr as a3,Qr as a4,Ur as a5,Or as a6,sr as a7,ya as a8,Nr as a9,Ar as aa,Wr as ab,na as ac,da as ad,ir as ae,Fr as af,oa as ag,Kr as ah,Lr as b,Ee as c,cr as d,br as e,mr as f,Mr as g,kr as h,sa as i,ta as j,Dr as k,hr as l,wr as m,vr as n,ra as o,$r as p,ia as q,dr as r,fr as s,tr as t,yr as u,Jr as v,ar as w,Tr as x,Pr as y,pa as z};
