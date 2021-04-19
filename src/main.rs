use anyhow::Result;
use reqwest::get;
use serde_json::Value;
use ansi_term::Color;
async fn youdao(words:&str)->Result<()> {
    let url = String::from("http://fanyi.youdao.com/openapi.do?keyfrom=node-fanyi&key=110811608&type=data&doctype=json&version=1.1&q=");
    let url = url+words;
    let resp = get(url).await?.text().await?;
    let result:Value = serde_json::from_str(resp.as_str())?;
    print_youdao(result)?;
    Ok(())
}

fn print_basic(data:&Value)->Result<()>{
    let query = data["query"].to_string().trim_matches('\"').to_string();
    let basic:&Value = &data["basic"];
    let suffix = Color::RGB(128,128,128).paint("  ~  fanyi.youdao.com");
    if let Value::Null = basic{
        println!("{}{}",query,suffix);
        return Ok(())
    }
    let phonetic = &basic["phonetic"];
    if let Value::Null = phonetic{
        println!("{}{}",query,suffix);
    }else{
        let phonetic = phonetic.to_string();
        let phonetic = phonetic.trim_matches('\"');
        let phonetic = "[ ".to_string()+phonetic+" ]";
        println!("{}   {} {}",query,Color::RGB(255,0,255).paint(phonetic),suffix)
    }
    print!("\n");
    let explains = &basic["explains"];
    if let Value::Array(vec) = explains{
        for item in vec{
            if let Value::String(exp) = item{
                let exp = exp.trim_matches('\"');
                let exp = Color::Green.paint(exp);
                println!("{}{}",Color::RGB(128,128,128).paint(" - "),exp);
            };
        }

    }
    Ok(())
}
fn print_sentences(data:&Value)->Result<()>{
    print!("\n");
    let query = data["query"].to_string().trim_matches('\"').to_string();
    let web = &data["web"];
    if let Value::Array(vec) = web{
        if vec.len() != 0{
            for (i,sen) in vec.iter().enumerate(){
                let key = &sen["key"];
                if let Value::String(key) = key{
                    let key = key.trim_matches('\"');
                    print!("{}. ",i+1);
                    highlight(key, &query)?;
                }
                let value = &sen["value"];
                if let Value::Array(vec) = value{
                    let result:Vec<String> = vec.iter().map(|item|item.to_string().trim_matches('\"').to_string()).collect();
                    let mut value = String::from("   ");
                    for item in result{
                        value.push_str(&item);
                        value.push_str(",");
                    }
                    println!("{}",Color::Cyan.paint(value));
                }
            }
        }
    }
    Ok(())
}

fn highlight(key:&str,query:&str)->Result<()>{
    let result:Vec<&str> =  key.split(query).collect();
    for (i,item) in result.iter().enumerate(){
        print!("{}",Color::RGB(128,128,128).paint(*item));
        if i != result.len()-1{
            print!("{}",Color::Yellow.paint(query));
        }else{
            print!("\n")
        }

    }
    Ok(())
}

fn print_youdao(data:Value)->Result<()>{
    print_basic(&data)?;
    print_sentences(&data)?;
    println!("\n   {}\n",Color::RGB(128,128,128).paint("------"));
    Ok(())
}

#[tokio::main]
async fn main()->Result<()> {
    let args:Vec<String> = std::env::args().collect();
    let words = &args[1..];
    let mut input  = String::from("");
    for word in words{
        input.push_str(word);
        input.push_str(" ");
    }
    // println!("{:?}",input);
    youdao(&input).await?;
    Ok(())
}
