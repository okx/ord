for (let time of document.body.getElementsByTagName('time')) {
  time.setAttribute('title', new Date(time.textContent));
}

let next = document.querySelector('a.next');
let prev = document.querySelector('a.prev');

window.addEventListener('keydown', e => {
  if (document.activeElement.tagName == 'INPUT') {
    return;
  }

  switch (e.key) {
    case 'ArrowRight':
      if (next) {
        window.location = next.href;
      }
      return;
    case 'ArrowLeft':
      if (prev) {
        window.location = prev.href;
      }
      return;
  }
});

const search = document.querySelector('form[action="/search"]');
const query = search.querySelector('input[name="query"]');

search.addEventListener('submit', (e) => {
  if (!query.value) {
    e.preventDefault();
  }
});

const names = ["/", "/goats", "/faq", "/contact"];
let links = document.getElementsByClassName("nav-link");
let index = names.indexOf(window.location.pathname);
links[index === -1 ? 0 : index].classList.add("active");

if (index > 0) {
  document.getElementById("search-form").classList.add("hidden");
}
