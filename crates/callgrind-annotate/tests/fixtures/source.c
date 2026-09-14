// source fixture
int main(void) {
  int x = work();
  x += work();
  return x;
}

int work(void) {
  return 7;
}
