const button = document.getElementById('clickBtn');
const couner = document.getElementById('counter');

let count = 0

button.addEventListener('click', function() {
  count++;
  counter.textContent = count;
  console.log(count)
});
