const contactForm = document.getElementById('contact-form');
const successMsg = document.getElementById('success-msg');
contactForm.addEventListener('submit', () => {
  // contactForm.classList.add('hidden');
  successMsg.classList.remove('hidden');
});