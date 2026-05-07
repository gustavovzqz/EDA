# Variáveis
BINARY = programa.exe
SRC = src/main.rs


# Comando padrão de compilação
build:
	rustc $(SRC) -o $(BINARY)

run: build
	@./$(BINARY) $(INPUT)

clean:
	rm -f $(BINARY)
