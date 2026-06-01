// 감정 발생 파이프라인 문서 — 공유 스크립트 (코드 펼치기 전역 토글)
document.addEventListener('DOMContentLoaded',()=>{
  const blocks=[...document.querySelectorAll('details.code')];
  if(!blocks.length)return;
  const bar=document.querySelector('.topbar .crumb');
  if(bar){
    const t=document.createElement('a');
    t.href='#';t.style.marginLeft='14px';t.textContent='⊕ 코드 전체 펼치기';
    let open=false;
    t.addEventListener('click',e=>{e.preventDefault();open=!open;
      blocks.forEach(b=>b.open=open);
      t.textContent=open?'⊖ 코드 전체 접기':'⊕ 코드 전체 펼치기';});
    bar.appendChild(t);
  }
});
