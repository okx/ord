var faqs = document.getElementsByClassName("faq-item");
for (var i = 0; i < faqs.length; i++) {
  faqs[i].addEventListener("click", function () {
    this.getElementsByClassName("faq-body")[0].classList.toggle("active");

    var arrowClassList = this.getElementsByClassName("arrow")[0].classList
    if(arrowClassList.contains('down')) {
      arrowClassList.remove('down');
      arrowClassList.add('up');
    } else {
      arrowClassList.remove('up');
      arrowClassList.add('down');
    }
  });
}