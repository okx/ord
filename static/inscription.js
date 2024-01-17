let text = document.getElementById("timestamp").innerText;
let date = new Date(text);
let options = { 
  year: 'numeric', 
  month: 'long', 
  day: 'numeric', 
  hour: 'numeric', 
  minute: 'numeric',
  timeZoneName: 'short'
};
let dateAndTime = date.toLocaleString('en-US', options);
document.getElementById("timestamp").innerHTML = dateAndTime;
document.getElementById("from-now").innerHTML = moment(date).fromNow();

const sliceAddress = (address) => address.slice(0, 5) + "..." + address.slice(-5);

let address = document.getElementById("address");
if (address) {
  document.getElementById("address").innerHTML = sliceAddress(address.innerText);
}

const getSize = (size) => {
  if (size < 1024) return size.toLocaleString('en-us') + " B";
  else return (size / 1024).toLocaleString('en-us', { maximumFractionDigits: 2 }) + " KB";
}

let size = parseInt(document.getElementById("size").innerText);
document.getElementById("size").innerHTML = getSize(size);

let hash = document.getElementById("hash").innerText;
document.getElementById("hash").innerHTML = sliceAddress(hash);

let block = parseInt(document.getElementById("block").innerText);
document.getElementById("block").innerHTML = block.toLocaleString('en-us');

let fee = parseInt(document.getElementById("fee").innerText);
document.getElementById("fee").innerHTML = fee.toLocaleString('en-us');

let transaction = document.getElementById("transaction").innerText;
document.getElementById("transaction").innerHTML = sliceAddress(transaction);

let sat_block = document.getElementById("block");
if (sat_block) {
  document.getElementById("block").innerHTML = parseInt(sat_block.innerText).toLocaleString('en-us');
}

const copyIcons = document.getElementsByClassName("copy-icon");
for (const copyIcon of copyIcons) {
  copyIcon.addEventListener('click', () => {
    navigator.clipboard.writeText(copyIcon.dataset.copy);
  });
}

const showMoreBtn = document.getElementById("show-more");
showMoreBtn.addEventListener('click', () => {
  document.getElementById("second-infos").classList.remove('hidden');
  showMoreBtn.classList.add('hidden');
});